//! PIA VPN Helm App
//! Implements HelmApp trait for pia-vpn

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::Result;
use std::collections::HashMap;

pub struct PiaVpn;

impl HelmApp for PiaVpn {
    fn chart_name(&self) -> &str {
        "pia-vpn"
    }

    fn namespace(&self) -> &str {
        "default"
    }

    fn release_name(&self) -> &str {
        "pia-vpn"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        let region = std::env::var("REGION").unwrap_or_default();
        let update_configs = std::env::var("UPDATE_CONFIGS")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);
        let proxy_port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "8888".to_string());
        let debug = std::env::var("DEBUG")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let is_dev = std::env::var("HALVOR_ENV")
            .map(|v| v.to_lowercase() == "development")
            .unwrap_or(false);
        let image_tag = if is_dev { "experimental" } else { "latest" };
        values.insert("image.tag".to_string(), image_tag.to_string());

        values.insert("vpn.region".to_string(), region);
        values.insert("vpn.updateConfigs".to_string(), update_configs.to_string());
        values.insert("vpn.proxyPort".to_string(), proxy_port);
        values.insert("vpn.debug".to_string(), debug.to_string());
        
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

