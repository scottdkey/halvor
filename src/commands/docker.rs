use crate::config;
use crate::services::docker;
use crate::utils::exec::Executor;
use anyhow::Result;

pub fn handle_docker(hostname: &str) -> Result<()> {
    let config = config::load_config()?;
    docker::install_docker(hostname, &config)?;
    Ok(())
}

/// Diagnose Docker daemon issues
pub fn diagnose_docker(hostname: Option<&str>) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");
    let exec = Executor::new(target_host, &config)?;

    docker::diagnostics::diagnose_docker(&exec, target_host)
}
