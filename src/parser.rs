use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug, PartialEq)]
pub struct OntMetrics {
    pub tx_power: f64,
    pub rx_power: f64,
    pub voltage: f64,
    pub bias_current: f64,
    pub temperature: f64,
}

pub fn parse_ont_metrics(html: &str) -> Result<OntMetrics> {
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

    let tx_power = tx_power_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse TX Power")?;
    let rx_power = rx_power_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse RX Power")?;
    let voltage = voltage_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Voltage")?;
    let temperature = temperature_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Temperature")?;
    let bias_current = bias_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse Bias Current")?;

    Ok(OntMetrics {
        tx_power,
        rx_power,
        voltage,
        bias_current,
        temperature,
    })
}

fn decode_hex_escapes(s: &str) -> String {
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
}
