use crate::config;
use crate::services::npm;
use anyhow::Result;

/// Handle NPM command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_npm(hostname: Option<&str>, compose_file: &str, service: Option<&str>) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");
    let rt = tokio::runtime::Runtime::new()?;
    if let Some(service_spec) = service {
        rt.block_on(npm::setup_single_proxy_host(
            target_host,
            service_spec,
            &config,
        ))?;
    } else if !compose_file.is_empty() {
        rt.block_on(npm::setup_proxy_hosts(target_host, compose_file, &config))?;
    } else {
        anyhow::bail!("Either --service or compose_file must be provided");
    }
    Ok(())
}
