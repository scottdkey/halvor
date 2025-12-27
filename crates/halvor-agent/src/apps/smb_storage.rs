//! SMB Storage Helm App
//! Implements HelmApp trait for smb-storage

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::Result;
use std::collections::HashMap;

pub struct SmbStorage;

impl HelmApp for SmbStorage {
    fn chart_name(&self) -> &str {
        "smb-storage"
    }

    fn namespace(&self) -> &str {
        "kube-system"
    }

    fn release_name(&self) -> &str {
        "smb-storage"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        // SMB storage uses SMB server config from environment
        // Values are typically set via values.yaml, not env vars
        
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

