//! Gitea Helm App
//! Implements HelmApp trait for gitea

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct Gitea;

impl HelmApp for Gitea {
    fn chart_name(&self) -> &str {
        "gitea"
    }

    fn namespace(&self) -> &str {
        "gitea"
    }

    fn release_name(&self) -> &str {
        "gitea"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        let domain = std::env::var("GITEA_DOMAIN")
            .or_else(|_| std::env::var("PUBLIC_DOMAIN").map(|d| format!("gitea.{}", d)))
            .or_else(|_| std::env::var("PRIVATE_DOMAIN").map(|d| format!("gitea.{}", d)))
            .context("GITEA_DOMAIN, PUBLIC_DOMAIN, or PRIVATE_DOMAIN environment variable not set")?;

        let root_url = std::env::var("GITEA_ROOT_URL")
            .unwrap_or_else(|_| format!("https://{}", domain));

        values.insert("domain".to_string(), domain.clone());
        values.insert("gitea.server.domain".to_string(), domain.clone());
        values.insert("gitea.server.rootUrl".to_string(), root_url);
        values.insert("ingress.hosts[0].host".to_string(), domain);
        
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

