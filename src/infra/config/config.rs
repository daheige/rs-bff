use crate::infra::errors::AppError;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app_debug: bool,
    pub app_port: u16,
    pub monitor_port: u16,
    pub log_level: String,
    pub graceful_wait_time: u64,
    pub services: Vec<ServiceConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub target: String,
}

impl Config {
    pub fn from_yaml(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn service_map(&self) -> HashMap<&str, &str> {
        self.services
            .iter()
            .map(|s| (s.name.as_str(), s.target.as_str()))
            .collect()
    }

    pub fn get_service_target(&self, name: &str) -> Result<String, AppError> {
        self.services
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.target.clone())
            .ok_or_else(|| AppError::Internal(format!("service '{}' not found in config", name)))
    }
}
