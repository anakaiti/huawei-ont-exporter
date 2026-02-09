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
        if let Ok(device_html) = self.fetch_device_info().await {
            let _ = parse_device_info_page(&device_html).map(|device_metrics| {
                result.device_model = device_metrics.model;
                result.serial_number = device_metrics.serial;
                result.software_version = device_metrics.version;
                result.uptime_seconds = device_metrics.uptime;
            });
        }
        
        if let Ok(wan_html) = self.fetch_wan_info().await {
            let _ = parse_wan_page(&wan_html).map(|wan_metrics| {
                result.wan_status = wan_metrics.status;
                result.wan_ip = wan_metrics.ip;
                result.wan_rx_bytes = wan_metrics.rx_bytes;
                result.wan_tx_bytes = wan_metrics.tx_bytes;
            });
        }
        
        if let Ok(lan_html) = self.fetch_lan_info().await {
            let _ = parse_lan_page(&lan_html).map(|client_metrics| {
                result.lan_clients_count = client_metrics.lan_count;
                result.wifi_clients_count = client_metrics.wifi_count;
            });
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
            "/html/amp/deviceinfo/deviceinfo.asp",
            "/html/amp/basic/deviceinfo.asp", 
            "/html/advance/deviceinfo/deviceinfo.asp",
            "/html/ssmp/deviceinfo/deviceinfo.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            let _resp = match self.client.get(&url).send().await {
                Ok(r) if r.status().is_success() => match r.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") => return Ok(html),
                    _ => continue,
                },
                _ => continue,
            };
        }
        
        Err(anyhow!("Could not fetch device info from any known path"))
    }

    // Fetch WAN/internet status page
    async fn fetch_wan_info(&self) -> Result<String> {
        debug!("Fetching WAN info");
        
        let paths = [
            "/html/amp/internet/internet.asp",
            "/html/amp/wan/wan.asp",
            "/html/advance/internet/internet.asp",
            "/html/bbsp/wan/wan.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            let _resp = match self.client.get(&url).send().await {
                Ok(r) if r.status().is_success() => match r.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") => return Ok(html),
                    _ => continue,
                },
                _ => continue,
            };
        }
        
        Err(anyhow!("Could not fetch WAN info from any known path"))
    }

    // Fetch LAN/WiFi clients page
    async fn fetch_lan_info(&self) -> Result<String> {
        debug!("Fetching LAN info");
        
        let paths = [
            "/html/amp/lanuser/lanuser.asp",
            "/html/amp/user/user.asp",
            "/html/advance/user/user.asp",
            "/html/bbsp/user/user.asp",
            "/html/amp/wlan/user.asp",
        ];
        
        for path in &paths {
            let url = format!("{}{}", self.base_url, path);
            let _resp = match self.client.get(&url).send().await {
                Ok(r) if r.status().is_success() => match r.text().await {
                    Ok(html) if !html.is_empty() && !html.contains("404") => return Ok(html),
                    _ => continue,
                },
                _ => continue,
            };
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
}

// Parse device info page
fn parse_device_info_page(html: &str) -> Result<DevicePageInfo> {
    use regex::Regex;
    
    let mut info = DevicePageInfo {
        model: None,
        serial: None,
        version: None,
        uptime: None,
    };
    
    // Model patterns
    let model_patterns = [
        r#"ProductClass["']?\s*[=:]\s*["']([^"']+)["']"#,
        r#"ModelName["']?\s*[=:]\s*["']([^"']+)["']"#,
        r#"new stDeviceInfo\([^)]*["']([A-Z]{2}\d{4,}[A-Z]?\d*)["']"#,
    ];
    
    for pattern in &model_patterns {
        if let Some(caps) = Regex::new(pattern).unwrap().captures(html) {
            info.model = Some(caps.get(1).unwrap().as_str().to_string());
            break;
        }
    }
    
    // Serial patterns
    if let Some(caps) = Regex::new(r#"SerialNumber["']?\s*[=:]\s*["']([^"']+)["']"#).unwrap().captures(html) {
        info.serial = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    // Version patterns
    if let Some(caps) = Regex::new(r#"SoftwareVersion["']?\s*[=:]\s*["']([^"']+)["']"#).unwrap().captures(html) {
        info.version = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    // Uptime patterns
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
    
    // Status patterns
    let status_patterns = [
        r#"WANStatus["']?\s*[=:]\s*["']([^"']+)["']"#,
        r#"ConnectionStatus["']?\s*[=:]\s*["']([^"']+)["']"#,
    ];
    
    for pattern in &status_patterns {
        if let Some(caps) = Regex::new(pattern).unwrap().captures(html) {
            wan.status = Some(caps.get(1).unwrap().as_str().to_string());
            break;
        }
    }
    
    // IP patterns
    if let Some(caps) = Regex::new(r#"WANIP["']?\s*[=:]\s*["'](\d+\.\d+\.\d+\.\d+)["']"#).unwrap().captures(html) {
        wan.ip = Some(caps.get(1).unwrap().as_str().to_string());
    }
    
    // Traffic patterns
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

// Parse LAN/WiFi clients page
fn parse_lan_page(html: &str) -> Result<ClientPageInfo> {
    use regex::Regex;
    
    let mut clients = ClientPageInfo {
        lan_count: None,
        wifi_count: None,
    };
    
    // Count patterns
    let lan_patterns = [
        r"LANClients[=:]\s*(\d+)",
        r"EthernetClients[=:]\s*(\d+)",
        r"new Array\(new stUserInfo\([^)]+\),\s*null\);",
    ];
    
    for pattern in &lan_patterns {
        if let Some(count) = Regex::new(pattern)
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u32>().ok())
        {
            clients.lan_count = Some(count);
            break;
        }
    }
    
    let wifi_patterns = [
        r"WiFiClients[=:]\s*(\d+)",
        r"WLANClients[=:]\s*(\d+)",
    ];
    
    for pattern in &wifi_patterns {
        if let Some(count) = Regex::new(pattern)
            .unwrap()
            .captures(html)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<u32>().ok())
        {
            clients.wifi_count = Some(count);
            break;
        }
    }
    
    // Try to count stUserInfo entries for more accurate count
    let user_re = Regex::new(r"new stUserInfo\(").unwrap();
    let user_count = user_re.find_iter(html).count() as u32;
    if user_count > 0 && clients.lan_count.is_none() {
        clients.lan_count = Some(user_count);
    }
    
    Ok(clients)
}
