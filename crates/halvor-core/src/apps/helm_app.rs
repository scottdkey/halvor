//! Helm Chart App Trait
//!
//! Defines the interface that all Helm chart apps must implement.
//! Helm chart apps are Kubernetes applications deployed via Helm charts.

use crate::config::EnvConfig;
use crate::apps::registry::AppDefinition;
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Trait for Helm chart applications
///
/// All apps that are deployed via Helm charts must implement this trait.
/// This provides a consistent interface for installing, upgrading, and managing
/// Helm chart-based applications.
pub trait HelmApp {
    /// Get the chart name (e.g., "portainer", "nginx-proxy-manager")
    fn chart_name(&self) -> &str;

    /// Get the default namespace for this chart
    fn namespace(&self) -> &str;

    /// Get the release name (defaults to chart name)
    fn release_name(&self) -> &str {
        self.chart_name()
    }

    /// Generate Helm values from environment variables
    ///
    /// This should extract relevant environment variables and convert them
    /// to Helm --set flags or a values file.
    fn generate_values(&self) -> Result<Vec<String>>;

    /// Install the Helm chart
    fn install(&self, hostname: &str, config: &EnvConfig) -> Result<()>;

    /// Upgrade the Helm chart
    fn upgrade(&self, hostname: &str, config: &EnvConfig) -> Result<()>;

    /// Uninstall the Helm chart
    fn uninstall(&self, hostname: &str, config: &EnvConfig) -> Result<()>;
}

/// Generic implementation of HelmApp for AppDefinition entries
///
/// This allows any AppDefinition with category HelmChart to be used as a HelmApp.
impl HelmApp for AppDefinition {
    fn chart_name(&self) -> &str {
        self.helm_chart_name.unwrap_or(self.name)
    }

    fn namespace(&self) -> &str {
        self.namespace.unwrap_or("default")
    }

    fn release_name(&self) -> &str {
        self.name
    }

    fn generate_values(&self) -> Result<Vec<String>> {
        // Use the existing Helm service function to generate values
        use crate::services::helm;
        
        // This is a bit of a hack - we need to access the private function
        // For now, we'll generate values using the chart name
        let chart = self.chart_name();
        let values_map = generate_values_from_env_internal(chart)?;
        
        // Convert to --set flags
        let mut flags = Vec::new();
        for (key, value) in values_map {
            // Escape special characters in values for --set
            let escaped_value = value.replace("'", "''").replace(",", "\\,");
            flags.push(format!("{}='{}'", key, escaped_value));
        }
        
        Ok(flags)
    }

    fn install(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        install_helm_app(self, hostname, config)
    }

    fn upgrade(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        upgrade_helm_app(self, hostname, config)
    }

    fn uninstall(&self, hostname: &str, config: &EnvConfig) -> Result<()> {
        uninstall_helm_app(self, hostname, config)
    }
}

/// Internal function to generate values from environment variables
/// This mirrors the logic in services::helm but is accessible here
fn generate_values_from_env_internal(chart: &str) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();

    match chart {
        "traefik-public" => {
            let domain = std::env::var("PUBLIC_DOMAIN")
                .context("PUBLIC_DOMAIN environment variable not set")?;
            let acme_email = std::env::var("ACME_EMAIL")
                .context("ACME_EMAIL environment variable not set")?;
            let cf_token = std::env::var("CF_DNS_API_TOKEN")
                .context("CF_DNS_API_TOKEN environment variable not set")?;

            values.insert("domain".to_string(), domain.clone());
            values.insert("acme.email".to_string(), acme_email);
            values.insert("acme.dnsToken".to_string(), cf_token);
            values.insert(
                "dashboard.domain".to_string(),
                format!("traefik.{}", domain),
            );
        }
        "traefik-private" => {
            let domain = std::env::var("PRIVATE_DOMAIN")
                .context("PRIVATE_DOMAIN environment variable not set")?;
            let acme_email = std::env::var("ACME_EMAIL")
                .context("ACME_EMAIL environment variable not set")?;
            let cf_token = std::env::var("CF_DNS_API_TOKEN")
                .context("CF_DNS_API_TOKEN environment variable not set")?;

            values.insert("domain".to_string(), domain.clone());
            values.insert("acme.email".to_string(), acme_email);
            values.insert("acme.dnsToken".to_string(), cf_token);
            values.insert(
                "dashboard.domain".to_string(),
                format!("traefik.{}", domain),
            );
        }
        "gitea" => {
            let domain = std::env::var("GITEA_DOMAIN")
                .or_else(|_| std::env::var("PUBLIC_DOMAIN").map(|d| format!("gitea.{}", d)))
                .or_else(|_| std::env::var("PRIVATE_DOMAIN").map(|d| format!("gitea.{}", d)))
                .context("GITEA_DOMAIN, PUBLIC_DOMAIN, or PRIVATE_DOMAIN environment variable not set")?;

            let root_url =
                std::env::var("GITEA_ROOT_URL").unwrap_or_else(|_| format!("https://{}", domain));

            values.insert("domain".to_string(), domain.clone());
            values.insert("gitea.server.domain".to_string(), domain.clone());
            values.insert("gitea.server.rootUrl".to_string(), root_url);
            values.insert("ingress.hosts[0].host".to_string(), domain);
        }
        "pia-vpn" => {
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
        }
        "halvor-server" => {
            let is_dev = std::env::var("HALVOR_ENV")
                .map(|v| v.to_lowercase() == "development")
                .unwrap_or(false);
            let image_tag = if is_dev { "experimental" } else { "latest" };
            values.insert("image.tag".to_string(), image_tag.to_string());
        }
        _ => {
            // For other charts, try to load common env vars
            if let Ok(domain) = std::env::var("PUBLIC_DOMAIN") {
                values.insert("domain".to_string(), domain);
            }
        }
    }

    Ok(values)
}

/// Helper function to install a Helm chart app
///
/// This is a convenience function that uses the Helm service to install
/// a Helm chart app. Individual apps can override the install method
/// if they need custom logic.
pub fn install_helm_app(
    app: &dyn HelmApp,
    hostname: &str,
    config: &EnvConfig,
) -> Result<()> {
    use crate::services::helm;

    helm::install_chart(
        hostname,
        app.chart_name(),
        Some(app.release_name()),
        Some(app.namespace()),
        None, // No values file - will generate from env vars
        &app.generate_values()?,
        None, // No external repo
        None, // No repo name
        config,
    )
}

/// Helper function to upgrade a Helm chart app
pub fn upgrade_helm_app(
    app: &dyn HelmApp,
    hostname: &str,
    config: &EnvConfig,
) -> Result<()> {
    use crate::services::helm;

    helm::upgrade_release(
        hostname,
        app.release_name(),
        None, // No values file
        &app.generate_values()?,
        config,
    )
}

/// Helper function to uninstall a Helm chart app
pub fn uninstall_helm_app(
    app: &dyn HelmApp,
    hostname: &str,
    config: &EnvConfig,
) -> Result<()> {
    use crate::services::helm;

    helm::uninstall_release(
        hostname,
        app.release_name(),
        false, // Don't skip confirmation
        config,
    )
}

