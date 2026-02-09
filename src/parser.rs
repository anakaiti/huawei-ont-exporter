use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, PartialEq, Default)]
pub struct OntMetrics {
    // Optical metrics
    pub tx_power: f64,
    pub rx_power: f64,
    pub voltage: f64,
    pub bias_current: f64,
    pub temperature: f64,

    // Device info metrics (optional)
    pub device_model: Option<String>,
    pub serial_number: Option<String>,
    pub software_version: Option<String>,
    pub uptime_seconds: Option<u64>,

    // WAN/Internet metrics (optional)
    pub wan_status: Option<String>,
    pub wan_ip: Option<String>,
    pub wan_rx_bytes: Option<u64>,
    pub wan_tx_bytes: Option<u64>,

    // LAN/WiFi metrics (optional)
    pub lan_clients_count: Option<u32>,
    pub wifi_clients_count: Option<u32>,
}

pub fn parse_ont_metrics(html: &str) -> Result<OntMetrics> {
    let mut metrics = OntMetrics::default();

    // Parse optical metrics
    parse_optical_metrics(html, &mut metrics)?;

    // Try to parse device info if available
    if let Ok(device_info) = parse_device_info(html) {
        metrics.device_model = device_info.model;
        metrics.serial_number = device_info.serial;
        metrics.software_version = device_info.version;
        metrics.uptime_seconds = device_info.uptime;
    }

    // Try to parse WAN metrics if available
    if let Ok(wan_info) = parse_wan_metrics(html) {
        metrics.wan_status = wan_info.status;
        metrics.wan_ip = wan_info.ip;
        metrics.wan_rx_bytes = wan_info.rx_bytes;
        metrics.wan_tx_bytes = wan_info.tx_bytes;
    }

    // Try to parse client counts if available
    if let Ok(client_info) = parse_client_metrics(html) {
        metrics.lan_clients_count = client_info.lan_count;
        metrics.wifi_clients_count = client_info.wifi_count;
    }

    Ok(metrics)
}

fn parse_optical_metrics(html: &str, metrics: &mut OntMetrics) -> Result<()> {
    // Look for: new stOpticInfo(..., "2.33", "-24.09", "3364", "47", "10", ...)
    // function definition: stOpticInfo(domain, LinkStatus, transOpticPower, revOpticPower, voltage, temperature, bias, ...)
    // Indices (0-based):
    // 2: transOpticPower (TX)
    // 3: revOpticPower (RX)
    // 4: voltage
    // 5: temperature
    // 6: bias

    let re = Regex::new(r"new stOpticInfo\(([^)]+)\)").unwrap();
    let caps = re
        .captures(html)
        .context("Failed to find stOpticInfo call")?;
    let args_str = caps.get(1).unwrap().as_str();

    // Split arguments by comma, considering they are quoted strings.
    // A simple split matches the example format sufficiently.
    let args: Vec<&str> = args_str.split(',').collect();

    if args.len() < 7 {
        return Err(anyhow::anyhow!("Not enough arguments in stOpticInfo call"));
    }

    // Helper to clean quotes and decode hex escapes
    let clean_arg = |s: &str| -> String {
        let s = s.trim().trim_matches('"');
        decode_hex_escapes(s)
    };

    let tx_power_str = clean_arg(args[2]);
    let rx_power_str = clean_arg(args[3]);
    let voltage_str = clean_arg(args[4]);
    let temperature_str = clean_arg(args[5]);
    let bias_str = clean_arg(args[6]);

    metrics.tx_power = tx_power_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse TX Power")?;
    metrics.rx_power = rx_power_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse RX Power")?;
    metrics.voltage = voltage_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Voltage")?;
    metrics.temperature = temperature_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Temperature")?;
    metrics.bias_current = bias_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Bias Current")?;

    Ok(())
}

#[derive(Debug, Default)]
struct DeviceInfo {
    model: Option<String>,
    serial: Option<String>,
    version: Option<String>,
    uptime: Option<u64>,
}

fn parse_device_info(html: &str) -> Result<DeviceInfo> {
    let mut info = DeviceInfo::default();

    // Try to find device model
    if let Some(caps) = Regex::new(r#"ProductClass["']?\s*[=:]\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(html)
    {
        info.model = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Try to find serial number
    if let Some(caps) = Regex::new(r#"SerialNumber["']?\s*[=:]\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(html)
    {
        info.serial = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Try to find software version
    if let Some(caps) = Regex::new(r#"SoftwareVersion["']?\s*[=:]\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(html)
    {
        info.version = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Try to find uptime (various formats)
    // Format 1: new stDeviceInfo(..., "12345", ...)
    info.uptime = Regex::new(r#"new stDeviceInfo\([^)]*"(\d+)"[^)]*\)"#)
        .unwrap()
        .captures(html)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    // Format 2: UpTime="12345"
    if info.uptime.is_none() {
        info.uptime = Regex::new(r#"UpTime[=:]\s*["']?(\d+)["']?"#)
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok());
    }

    Ok(info)
}

#[derive(Debug, Default)]
struct WanMetrics {
    status: Option<String>,
    ip: Option<String>,
    rx_bytes: Option<u64>,
    tx_bytes: Option<u64>,
}

fn parse_wan_metrics(html: &str) -> Result<WanMetrics> {
    let mut wan = WanMetrics::default();

    // Try to find WAN status
    if let Some(caps) = Regex::new(r#"WANStatus["']?\s*[=:]\s*["']([^"']+)["']"#)
        .unwrap()
        .captures(html)
    {
        wan.status = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Try to find WAN IP
    if let Some(caps) = Regex::new(r#"WANIP["']?\s*[=:]\s*["'](\d+\.\d+\.\d+\.\d+)["']"#)
        .unwrap()
        .captures(html)
    {
        wan.ip = Some(caps.get(1).unwrap().as_str().to_string());
    }

    // Try to find RX/TX bytes
    wan.rx_bytes = Regex::new(r"RXBytes[=:]\s*(\d+)")
        .unwrap()
        .captures(html)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    wan.tx_bytes = Regex::new(r"TXBytes[=:]\s*(\d+)")
        .unwrap()
        .captures(html)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok());

    Ok(wan)
}

#[derive(Debug, Default)]
struct ClientMetrics {
    lan_count: Option<u32>,
    wifi_count: Option<u32>,
}

fn parse_client_metrics(html: &str) -> Result<ClientMetrics> {
    let clients = ClientMetrics {
        lan_count: Regex::new(r"LANClients[=:]\s*(\d+)")
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u32>().ok()),
        wifi_count: Regex::new(r"WiFiClients[=:]\s*(\d+)")
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u32>().ok()),
    };

    Ok(clients)
}

pub fn decode_hex_escapes(s: &str) -> String {
    let mut output = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&'x') = chars.peek() {
                chars.next(); // consume 'x'
                              // Read next two chars
                let h1 = chars.next().unwrap_or('0');
                let h2 = chars.next().unwrap_or('0');
                let hex_str = format!("{}{}", h1, h2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    output.push(byte as char);
                } else {
                    // If parsing fails, push original chars (fallback)
                    output.push('\\');
                    output.push('x');
                    output.push(h1);
                    output.push(h2);
                }
            } else {
                output.push(c);
            }
        } else {
            output.push(c);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metrics_js() {
        let html = r#"
        var opticInfos = new Array(new stOpticInfo("InternetGatewayDevice.X_HW_DEBUG.AMP.Optic","ok","\x202\x2e33","\x2d24\x2e09","3364","47","10","\x2d\x2d","\x2d\x2d","HUAWEI\x20\x20\x20\x20\x20\x20\x20\x20\x20","2416R080776AS\x20\x20","240529","1310","1490","20","0"),null);
        "#;

        let metrics = parse_ont_metrics(html).unwrap();

        assert_eq!(metrics.tx_power, 2.33);
        assert_eq!(metrics.rx_power, -24.09);
        assert_eq!(metrics.voltage, 3364.0);
        assert_eq!(metrics.temperature, 47.0);
        assert_eq!(metrics.bias_current, 10.0);
    }

    #[test]
    fn test_parse_device_info() {
        let html = r#"
        var deviceInfo = {
            ProductClass: "HG8145V5",
            SerialNumber: "4857544345AABBCC",
            SoftwareVersion: "V5R019C00S180",
            UpTime: "86400"
        };
        "#;

        let info = parse_device_info(html).unwrap();
        assert_eq!(info.model, Some("HG8145V5".to_string()));
        assert_eq!(info.serial, Some("4857544345AABBCC".to_string()));
        assert_eq!(info.version, Some("V5R019C00S180".to_string()));
        assert_eq!(info.uptime, Some(86400));
    }
}
