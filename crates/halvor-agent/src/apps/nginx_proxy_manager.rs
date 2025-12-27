//! Nginx Proxy Manager Helm App
//! Implements HelmApp trait for nginx-proxy-manager

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::Result;
use std::collections::HashMap;

pub struct NginxProxyManager;

impl HelmApp for NginxProxyManager {
    fn chart_name(&self) -> &str {
        "nginx-proxy-manager"
    }

    fn namespace(&self) -> &str {
        "default"
    }

    fn release_name(&self) -> &str {
        "nginx-proxy-manager"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        // NPM doesn't require special env vars, uses default values
        // But we can add custom values if needed
        
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

