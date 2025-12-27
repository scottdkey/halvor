use halvor_core::config::EnvConfig;
use halvor_core::services::host;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct LoginRequest {
    identity: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Debug, Serialize)]
struct ProxyHostRequest {
    domain_names: Vec<String>,
    forward_scheme: String,
    forward_host: String,
    forward_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssl_forced: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    certificate_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_list_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    advanced_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locations: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    block_exploits: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    caching_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allow_websocket_upgrade: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    http2_support: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProxyHost {
    id: u32,
    domain_names: Vec<String>,
    // forward_scheme: String,
    // forward_host: String,
    // forward_port: u16,
}

pub async fn setup_proxy_hosts(
    hostname: &str,
    compose_file: &str,
    _config: &EnvConfig,
) -> Result<()> {
    let host_config = host::get_host_config_or_error(hostname)?;

    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(hostname) = &host_config.hostname {
        hostname.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    println!(
        "Setting up Nginx Proxy Manager hosts for {} ({})...",
        hostname, target_host
    );
    println!();

    // Read compose file
    let halvor_dir = halvor_core::config::find_halvor_dir()?;
    let compose_path = halvor_dir.join("compose").join(compose_file);

    if !compose_path.exists() {
        anyhow::bail!("Compose file not found: {}", compose_path.display());
    }

    let compose_content = std::fs::read_to_string(&compose_path)
        .with_context(|| format!("Failed to read compose file: {}", compose_path.display()))?;

    // Parse compose file to extract services with ports
    let services = parse_compose_services(&compose_content)?;

    if services.is_empty() {
        println!("No services with exposed ports found in compose file");
        return Ok(());
    }

    println!("Found {} service(s) with exposed ports:", services.len());
    for (name, port) in &services {
        println!("  - {}:{}", name, port);
    }
    println!();

    // Get NPM credentials from config
    let npm_url =
        halvor_core::config::get_npm_url().unwrap_or_else(|| format!("https://{}:81", target_host));
    let npm_username = halvor_core::config::get_npm_username().context("NPM_USERNAME not set in .env")?;
    let npm_password = halvor_core::config::get_npm_password().context("NPM_PASSWORD not set in .env")?;

    // Login to NPM API
    let token = login_to_npm(&npm_url, &npm_username, &npm_password)
        .await
        .context("Failed to login to Nginx Proxy Manager")?;
    println!("✓ Authenticated with Nginx Proxy Manager");
    println!();

    // Get existing proxy hosts
    let existing_hosts = get_proxy_hosts(&npm_url, &token)
        .await
        .context("Failed to get existing proxy hosts")?;
    println!("Found {} existing proxy host(s)", existing_hosts.len());
    println!();

    // Create or update proxy hosts
    for (service_name, port) in &services {
        let domain = format!("{}.local", service_name); // Default domain pattern
        println!("Setting up proxy host for {}...", service_name);

        // Check if proxy host already exists
        let existing = existing_hosts
            .iter()
            .find(|h| h.domain_names.contains(&domain));

        if let Some(existing_host) = existing {
            println!(
                "  Proxy host already exists (ID: {}), skipping",
                existing_host.id
            );
            continue;
        }

        // Create new proxy host
        match create_proxy_host(&npm_url, &token, &domain, &target_host, *port).await {
            Ok(id) => {
                println!("  ✓ Created proxy host (ID: {})", id);
                println!("    Domain: {}", domain);
                println!("    Forward: http://{}:{}", target_host, port);
            }
            Err(e) => {
                println!("  ✗ Failed to create proxy host: {}", e);
            }
        }
        println!();
    }

    println!("✓ Proxy host setup complete");
    Ok(())
}

fn parse_compose_services(compose_content: &str) -> Result<HashMap<String, u16>> {
    use yaml_rust::YamlLoader;

    let docs = YamlLoader::load_from_str(compose_content).context("Failed to parse YAML")?;

    if docs.is_empty() {
        return Ok(HashMap::new());
    }

    let doc = &docs[0];
    let mut services = HashMap::new();

    if let Some(services_node) = doc["services"].as_hash() {
        for (service_name, service_config) in services_node {
            if let Some(name_str) = service_name.as_str() {
                if let Some(ports_node) = service_config["ports"].as_vec() {
                    // Get the first port mapping
                    if let Some(first_port) = ports_node.first() {
                        if let Some(port_str) = first_port.as_str() {
                            // Parse "8989:8989" format
                            if let Some(host_port) = port_str.split(':').next() {
                                if let Ok(port) = host_port.parse::<u16>() {
                                    services.insert(name_str.to_string(), port);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(services)
}

async fn login_to_npm(url: &str, username: &str, password: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // For self-signed certs
        .cookie_store(true)
        .build()?;

    let login_url = format!("{}/api/tokens", url);
    let login_req = LoginRequest {
        identity: username.to_string(),
        password: password.to_string(),
    };

    let response = client
        .post(&login_url)
        .json(&login_req)
        .send()
        .await
        .context("Failed to connect to Nginx Proxy Manager")?;

    if !response.status().is_success() {
        anyhow::bail!("Login failed: {}", response.status());
    }

    let login_resp: LoginResponse = response
        .json()
        .await
        .context("Failed to parse login response")?;

    Ok(login_resp.token)
}

async fn get_proxy_hosts(url: &str, token: &str) -> Result<Vec<ProxyHost>> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let api_url = format!("{}/api/nginx/proxy-hosts", url);

    let response = client
        .get(&api_url)
        .bearer_auth(token)
        .send()
        .await
        .context("Failed to fetch proxy hosts")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get proxy hosts: {}", response.status());
    }

    let hosts: Vec<ProxyHost> = response
        .json()
        .await
        .context("Failed to parse proxy hosts response")?;

    Ok(hosts)
}

async fn create_proxy_host(
    url: &str,
    token: &str,
    domain: &str,
    forward_host: &str,
    forward_port: u16,
) -> Result<u32> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let api_url = format!("{}/api/nginx/proxy-hosts", url);

    let proxy_req = ProxyHostRequest {
        domain_names: vec![domain.to_string()],
        forward_scheme: "http".to_string(),
        forward_host: forward_host.to_string(),
        forward_port,
        ssl_forced: Some(false),
        certificate_id: None,
        access_list_id: None,
        advanced_config: None,
        locations: None,
        block_exploits: Some(false),
        caching_enabled: Some(false),
        allow_websocket_upgrade: Some(true),
        http2_support: Some(false),
    };

    let response = client
        .post(&api_url)
        .bearer_auth(token)
        .json(&proxy_req)
        .send()
        .await
        .context("Failed to create proxy host")?;

    let status = response.status();
    let response_text = response.text().await.unwrap_or_default();

    if !status.is_success() {
        anyhow::bail!(
            "Failed to create proxy host: {} - {}",
            status,
            response_text
        );
    }

    let created: serde_json::Value =
        serde_json::from_str(&response_text).context("Failed to parse create response")?;

    let id = created["id"].as_u64().context("Response missing ID")? as u32;

    Ok(id)
}

pub async fn setup_single_proxy_host(hostname: &str, service_spec: &str) -> Result<()> {
    // Parse service spec: "servicename:port" or "servicename" (defaults to port from common services)
    let (service_name, port) = if let Some((name, port_str)) = service_spec.split_once(':') {
        let port = port_str
            .parse::<u16>()
            .with_context(|| format!("Invalid port: {}", port_str))?;
        (name, port)
    } else {
        // Check for common service names
        let port = match service_spec.to_lowercase().as_str() {
            "portainer" => 9000,
            "npm" | "nginx-proxy-manager" => 81,
            _ => anyhow::bail!(
                "Unknown service '{}'. Use format 'servicename:port' (e.g., 'portainer:9000')",
                service_spec
            ),
        };
        (service_spec, port)
    };

    let host_config = host::get_host_config_or_error(hostname)?;

    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(hostname) = &host_config.hostname {
        hostname.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    println!(
        "Setting up proxy host for {} on {}...",
        service_name, hostname
    );
    println!();

    // Get NPM credentials from config
    let npm_url =
        halvor_core::config::get_npm_url().unwrap_or_else(|| format!("https://{}:81", target_host));
    let npm_username = halvor_core::config::get_npm_username().context("NPM_USERNAME not set in .env")?;
    let npm_password = halvor_core::config::get_npm_password().context("NPM_PASSWORD not set in .env")?;

    // Login to NPM API
    let token = login_to_npm(&npm_url, &npm_username, &npm_password)
        .await
        .context("Failed to login to Nginx Proxy Manager")?;
    println!("✓ Authenticated with Nginx Proxy Manager");
    println!();

    // Get existing proxy hosts
    let existing_hosts = get_proxy_hosts(&npm_url, &token)
        .await
        .context("Failed to get existing proxy hosts")?;

    let domain = format!("{}.local", service_name);

    // Check if proxy host already exists
    let existing = existing_hosts
        .iter()
        .find(|h| h.domain_names.contains(&domain));

    if let Some(existing_host) = existing {
        println!(
            "Proxy host already exists (ID: {}) for {}",
            existing_host.id, domain
        );
        println!("  Domain: {}", domain);
        return Ok(());
    }

    // Create new proxy host
    match create_proxy_host(&npm_url, &token, &domain, &target_host, port).await {
        Ok(id) => {
            println!("✓ Created proxy host (ID: {})", id);
            println!("  Domain: {}", domain);
            println!("  Forward: http://{}:{}", target_host, port);
        }
        Err(e) => {
            anyhow::bail!("Failed to create proxy host: {}", e);
        }
    }

    Ok(())
}

