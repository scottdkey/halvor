use crate::config;
use crate::services::docker;
use anyhow::Result;

pub fn handle_docker(hostname: &str) -> Result<()> {
    let config = config::load_config()?;
    docker::install_docker(hostname, &config)?;
    Ok(())
}
