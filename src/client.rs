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
        
        // Ensure logout happens even if fetch fails
        let result = self.fetch_optical_info().await;
        
        let logout_res = self.logout().await;
        if let Err(e) = logout_res {
            error!("Logout failed: {}", e);
        }

        result
    }

    async fn get_login_token(&self) -> Result<String> {
        let url = format!("{}/asp/GetRandCount.asp", self.base_url);
        
        // Request headers required to get the token
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
        
        // Remove BOM if present (EF BB BF)
        let token = text.trim_start_matches('\u{feff}').trim();
        
        Ok(token.to_string())
    }

    async fn login(&self) -> Result<()> {
        debug!("Logging in to {}", self.base_url);
        
        // Step 1: GET / to initialize session/cookies
        let _ = self.client.get(&self.base_url).send().await;

        // Step 2: Get Token
        let token = self.get_login_token().await.context("Failed to get login token")?;
        debug!("Got login token: {}", token);

        // Step 3: Prepare credentials
        // Password must be Base64 encoded
        let password_base64 = BASE64_STANDARD.encode(&self.pass);
        
        let params = [
            ("UserName", self.user.as_str()),
            ("PassWord", password_base64.as_str()),
            ("Language", "english"),
            ("x.X_HW_Token", token.as_str()),
        ];
        
        // Step 4: Login POST
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
        // Verify success
        // Success typically returns a page with a script redirection or sets a cookie
        // Failure often returns the login page again or an error code
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

    async fn logout(&self) -> Result<()> {
        debug!("Logging out");
        // Logout URL from HAR: logout.cgi?RequestFile=html/logout.html
        let url = format!("{}/logout.cgi?RequestFile=html/logout.html", self.base_url);
        let _ = self.client.get(&url).send().await;
        Ok(())
    }
}
