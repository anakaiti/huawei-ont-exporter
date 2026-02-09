use reqwest::Client;
use anyhow::{Result, Context, anyhow};
use tracing::{error, debug};
use std::time::Duration;
use crate::parser::{parse_ont_metrics, OntMetrics};
use base64::prelude::*;

pub struct OntClient {
    client: Client,
    base_url: String,
    user: String,
    pass: String,
}

impl OntClient {
    pub fn new(url: &str, user: &str, pass: &str) -> Result<Self> {
        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            base_url: url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
        })
    }

    pub async fn scrape_metrics(&self) -> Result<OntMetrics> {
        self.login().await.context("Failed to login")?;
        
        // Scrape optical metrics (primary)
        let mut result = self.fetch_optical_info().await
            .context("Failed to fetch optical info")?;
        
        // Try to fetch additional metrics (optional - don't fail if unavailable)
        match self.fetch_device_info().await {
            Ok(device_html) => {
                match parse_device_info_page(&device_html) {
                    Ok(device_metrics) => {
                        debug!("Device info parsed successfully");
                        result.device_model = device_metrics.model;
                        result.serial_number = device_metrics.serial;
                        result.software_version = device_metrics.version;
                        result.uptime_seconds = device_metrics.uptime;
                        result.hardware_version = device_metrics.hardware_version;
                        result.mac_address = device_metrics.mac;
                    }
                    Err(e) => debug!("Failed to parse device info: {}", e),
                }
            }
            Err(e) => debug!("Failed to fetch device info: {}", e),
        }
        
        match self.fetch_wan_info().await {
            Ok(wan_html) => {
                match parse_wan_page(&wan_html) {
                    Ok(wan_metrics) => {
                        debug!("WAN info parsed successfully");
                        result.wan_status = wan_metrics.status;
                        result.wan_ip = wan_metrics.ip;
                        result.wan_rx_bytes = wan_metrics.rx_bytes;
                        result.wan_tx_bytes = wan_metrics.tx_bytes;
                    }
                    Err(e) => debug!("Failed to parse WAN info: {}", e),
                }
            }
            Err(e) => debug!("Failed to fetch WAN info: {}", e),
        }
        
        match self.fetch_lan_info().await {
            Ok(lan_html) => {
                match parse_lan_page(&lan_html) {
                    Ok(client_metrics) => {
                        debug!("LAN info parsed successfully");
                        result.lan_clients_count = client_metrics.lan_count;
                        result.wifi_clients_count = client_metrics.wifi_count;
                        result.total_clients_count = client_metrics.total_count;
                    }
                    Err(e) => debug!("Failed to parse LAN info: {}", e),
                }
            }
            Err(e) => debug!("Failed to fetch LAN info: {}", e),
        }
        
        let logout_res = self.logout().await;
        if let Err(e) = logout_res {
            error!("Logout failed: {}", e);
        }

        Ok(result)
    }

    async fn get_login_token(&self) -> Result<String> {
        let url = format!("{}/asp/GetRandCount.asp", self.base_url);
        
        let resp = self.client.post(&url)
            .header("Referer", format!("{}/", self.base_url))
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", &self.base_url)
            .send()
            .await
            .context("Failed to send GetRandCount request")?;
            
        if !resp.status().is_success() {
             return Err(anyhow!("GetRandCount failed with status: {}", resp.status()));
        }

        let text = resp.text().await.context("Failed to get GetRandCount response text")?;
        
        let token = text.trim_start_matches('\u{feff}').trim();
        
        Ok(token.to_string())
    }

    async fn login(&self) -> Result<()> {
        debug!("Logging in to {}", self.base_url);
        
        let _ = self.client.get(&self.base_url).send().await;

        let token = self.get_login_token().await.context("Failed to get login token")?;
        debug!("Got login token: {}", token);

        let password_base64 = BASE64_STANDARD.encode(&self.pass);
        
        let params = [
            ("UserName", self.user.as_str()),
            ("PassWord", password_base64.as_str()),
            ("Language", "english"),
            ("x.X_HW_Token", token.as_str()),
        ];
        
        let login_url = format!("{}/login.cgi", self.base_url);
        let resp = self.client.post(&login_url)
            .header("Referer", format!("{}/", self.base_url))
            .form(&params)
            .send()
            .await
            .context("Failed to send login request")?;

        if !resp.status().is_success() {
             return Err(anyhow!("Login request failed with status: {}", resp.status()));
        }
             
        let text = resp.text().await?;
        if text.contains("login.asp") && !text.contains("top.location.replace") {
             return Err(anyhow!("Login failed: received login page"));
        }
        
        debug!("Login successful");
        Ok(())
    }

    async fn fetch_optical_info(&self) -> Result<OntMetrics> {
        debug!("Fetching optical info");
        
        let url = format!("{}/html/amp/opticinfo/opticinfo.asp", self.base_url);
        let resp = self.client.get(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow!("Failed to fetch metrics page: {}", resp.status()));
        }
        
        let html = resp.text().await?;
        parse_ont_metrics(&html).context("Failed to parse metrics")
    }

    // Fetch device information page
    async fn fetch_device_info(&self) -> Result<String> {
        debug!("Fetching device info");
        
        // Common paths for device info on Huawei ONTs
        let paths = [
            "/html/ssmp/deviceinfo/deviceinfo.asp",
            "/html/amp/deviceinfo/deviceinfo.asp",
            "/html/amp/basic/deviceinfo.asp", 
            "/html/advance/deviceinfo/deviceinfo.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") && html.contains("stDeviceInfo") => {
                        return Ok(html);
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
        
        Err(anyhow!("Could not fetch device info from any known path"))
    }

    // Fetch WAN/internet status page
    async fn fetch_wan_info(&self) -> Result<String> {
        debug!("Fetching WAN info");
        
        let paths = [
            "/html/bbsp/waninfo/waninfo.asp",
            "/html/amp/internet/internet.asp",
            "/html/amp/wan/wan.asp",
            "/html/advance/internet/internet.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") => {
                        return Ok(html);
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
        
        Err(anyhow!("Could not fetch WAN info from any known path"))
    }

    // Fetch LAN/WiFi clients page
    async fn fetch_lan_info(&self) -> Result<String> {
        debug!("Fetching LAN info");
        
        let paths = [
            "/html/bbsp/common/GetLanUserDevInfo.asp",
            "/html/amp/lanuser/lanuser.asp",
            "/html/amp/user/user.asp",
            "/html/advance/user/user.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            match self.client.get(&url).send().await {
                Ok(resp) => match resp.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") => {
                        return Ok(html);
                    }
                    _ => continue,
                },
                _ => continue,
            }
        }
        
        Err(anyhow!("Could not fetch LAN info from any known path"))
    }

    async fn logout(&self) -> Result<()> {
        debug!("Logging out");
        let url = format!("{}/logout.cgi?RequestFile=html/logout.html", self.base_url);
        let _ = self.client.get(&url).send().await;
        Ok(())
    }
}

// Helper structs for additional page parsing

pub struct DevicePageInfo {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub version: Option<String>,
    pub hardware_version: Option<String>,
    pub mac: Option<String>,
    pub uptime: Option<u64>,
}

pub struct WanPageInfo {
    pub status: Option<String>,
    pub ip: Option<String>,
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
}

pub struct ClientPageInfo {
    pub lan_count: Option<u32>,
    pub wifi_count: Option<u32>,
    pub total_count: Option<u32>,
}

// Parse device info page
fn parse_device_info_page(html: &str) -> Result<DevicePageInfo> {
    use regex::Regex;
    use crate::parser::decode_hex_escapes;
    
    let mut info = DevicePageInfo {
        model: None,
        serial: None,
        version: None,
        hardware_version: None,
        mac: None,
        uptime: None,
    };
    
    // Parse stDeviceInfo array: new stDeviceInfo("domain","serial","hardware","software","model",...)
    // Format: "485754439A54FCAF","26AD\x2eA","V5R020C10S254","HG8145V5",...
    if let Some(caps) = Regex::new(r#"new stDeviceInfo\(([^)]+)\)"#).unwrap().captures(html) {
        let args_str = caps.get(1).unwrap().as_str();
        let args: Vec<&str> = args_str.split(',').collect();
        
        if args.len() >= 2 {
            info.serial = Some(decode_hex_escapes(args[1].trim().trim_matches('"')));
        }
        if args.len() >= 3 {
            info.hardware_version = Some(decode_hex_escapes(args[2].trim().trim_matches('"')));
        }
        if args.len() >= 4 {
            info.version = Some(decode_hex_escapes(args[3].trim().trim_matches('"')));
        }
        if args.len() >= 5 {
            info.model = Some(decode_hex_escapes(args[4].trim().trim_matches('"')));
        }
        if args.len() >= 8 {
            // MAC address is at position 8 (index 7)
            let mac_encoded = args[7].trim().trim_matches('"');
            let mac = decode_hex_escapes(mac_encoded);
            info.mac = Some(mac);
        }
    }
    
    // Uptime patterns (from optical info or separate calls)
    let uptime_patterns = [
        r"UpTime[=:]\s*(\d+)",
        r"new stDeviceInfo\([^)]*(\d{5,})[^)]*\)",
    ];
    
    for pattern in &uptime_patterns {
        if let Some(uptime) = Regex::new(pattern)
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok())
        {
            info.uptime = Some(uptime);
            break;
        }
    }
    
    Ok(info)
}

// Parse WAN info page
fn parse_wan_page(html: &str) -> Result<WanPageInfo> {
    use regex::Regex;
    
    let mut wan = WanPageInfo {
        status: None,
        ip: None,
        rx_bytes: None,
        tx_bytes: None,
    };
    
    // Look for WAN status in CurrentWan object
    if let Some(caps) = Regex::new(r#"CurrentWan\.Status\s*=\s*['"]([^'"]+)['"]"#).unwrap().captures(html) {
        wan.status = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    // Look for IPv4 IP address
    if let Some(caps) = Regex::new(r#"IPv4IPAddress\s*=\s*['"](\d+\.\d+\.\d+\.\d+)['"]"#).unwrap().captures(html) {
        wan.ip = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    // Alternative: from AddressList
    if wan.ip.is_none()
        && let Some(caps) = Regex::new(r#"IPAddress['"]\s*[=:]\s*['"](\d+\.\d+\.\d+\.\d+)['"]"#).unwrap().captures(html)
    {
        wan.ip = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    Ok(wan)
}

// Parse LAN/WiFi clients page
fn parse_lan_page(html: &str) -> Result<ClientPageInfo> {
    use regex::Regex;
    
    let mut clients = ClientPageInfo {
        lan_count: None,
        wifi_count: None,
        total_count: None,
    };
    
    // Count USERDevice entries in the array
    let user_device_re = Regex::new(r"new\s+(?:USERDevice|USERDeviceNew)\(").unwrap();
    let total_count = user_device_re.find_iter(html).count() as u32;
    
    if total_count > 0 {
        clients.total_count = Some(total_count);
        
        // Parse actual array entries to count LAN vs WiFi
        // Array entries look like: new USERDevice("...","...","...","LAN2",...)
        // The Port is the 4th parameter (index 3)
        let lan_count = Regex::new(r#"new\s+(?:USERDevice|USERDeviceNew)\([^)]*"(LAN\d*)"[^)]*\)"#)
            .unwrap()
            .find_iter(html)
            .count() as u32;
        let wifi_count = Regex::new(r#"new\s+(?:USERDevice|USERDeviceNew)\([^)]*"(SSID\d*)"[^)]*\)"#)
            .unwrap()
            .find_iter(html)
            .count() as u32;
        
        if lan_count > 0 {
            clients.lan_count = Some(lan_count);
        }
        if wifi_count > 0 {
            clients.wifi_count = Some(wifi_count);
        }
    }
    
    Ok(clients)
}
