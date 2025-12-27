//! Traefik Public Helm App
//! Implements HelmApp trait for traefik-public

use crate::apps::helm_app::HelmApp;
use halvor_core::config::EnvConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct TraefikPublic;

impl HelmApp for TraefikPublic {
    fn chart_name(&self) -> &str {
        "traefik-public"
    }

    fn namespace(&self) -> &str {
        "traefik"
    }

    fn release_name(&self) -> &str {
        "traefik-public"
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        let mut values = HashMap::new();
        
        let domain = std::env::var("PUBLIC_TLD")
            .context("PUBLIC_TLD environment variable not set")?;
        let acme_email = std::env::var("ACME_EMAIL")
            .context("ACME_EMAIL environment variable not set")?;
        let cf_token = std::env::var("CF_DNS_API_TOKEN")
            .context("CF_DNS_API_TOKEN environment variable not set")?;

        values.insert("domain".to_string(), domain.clone());
        values.insert("acme.email".to_string(), acme_email);
        values.insert("acme.dnsToken".to_string(), cf_token);
        values.insert("dashboard.domain".to_string(), format!("traefik.{}", domain));
        
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

