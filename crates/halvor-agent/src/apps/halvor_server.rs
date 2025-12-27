//! Halvor Server Helm App
//! Implements HelmApp trait for halvor-server

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::Result;
use std::collections::HashMap;

pub struct HalvorServer;

impl HelmApp for HalvorServer {
    fn chart_name(&self) -> &str {
        "halvor-server"
    }

    fn namespace(&self) -> &str {
        "default"
    }

    fn release_name(&self) -> &str {
        "halvor-server"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        let is_dev = std::env::var("HALVOR_ENV")
            .map(|v| v.to_lowercase() == "development")
            .unwrap_or(false);
        let image_tag = if is_dev { "experimental" } else { "latest" };
        values.insert("image.tag".to_string(), image_tag.to_string());
        
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

