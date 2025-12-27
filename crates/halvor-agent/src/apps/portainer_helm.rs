//! Portainer Helm App
//! Implements HelmApp trait for portainer

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::Result;
use std::collections::HashMap;

pub struct Portainer;

impl HelmApp for Portainer {
    fn chart_name(&self) -> &str {
        "portainer"
    }

    fn namespace(&self) -> &str {
        "default"
    }

    fn release_name(&self) -> &str {
        "portainer"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        // Portainer uses default values, but can be customized via env vars
        // Try to load common env vars
        if let Ok(domain) = std::env::var("PUBLIC_TLD") {
            values.insert("domain".to_string(), domain);
        }
        
        // Convert to --set flags
        let mut flags = Vec::new();
        for (key, value) in values {
            let escaped_value = value.replace("'", "''").replace(",", "\\,");
            flags.push(format!("{}='{}'", key, escaped_value));
        }
        
        Ok(flags)
    }

    fn install(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        crate::apps::helm_app::install_helm_app(self, hostname, config)
    }

    fn upgrade(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        crate::apps::helm_app::upgrade_helm_app(self, hostname, config)
    }

    fn uninstall(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        crate::apps::helm_app::uninstall_helm_app(self, hostname, config)
    }
}

