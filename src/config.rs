use toml;
use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiConfig {
    pub api_token: String,
    pub zone_id: String,
    pub domain: String,
}

impl ApiConfig {
    pub fn load_from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ApiConfig = toml::from_str(&content)?;
        Ok(config)
    }
}