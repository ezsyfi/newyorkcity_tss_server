use std::collections::HashMap;
use super::super::Result;
pub struct HcmcConfig {
    pub endpoint: String,
}

pub fn get_hcmc_host(settings: HashMap<String, String>) -> Result<HcmcConfig> {
    let hcmc_ip = settings
        .get("HCMC_HOST")
        .unwrap_or(&"http://localhost".to_string())
        .to_owned();

    let hcmc_port = settings
        .get("HCMC_PORT")
        .unwrap_or(&"8080".to_string())
        .to_owned();

    Ok(HcmcConfig {
        endpoint: format!("{}:{}", hcmc_ip, hcmc_port)
    })
}