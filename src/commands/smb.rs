use crate::config;
use crate::services::smb;
use anyhow::Result;

/// Handle SMB command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_smb(hostname: Option<&str>, uninstall: bool) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");
    if uninstall {
        smb::uninstall_smb_mounts(target_host, &config)?;
    } else {
        smb::setup_smb_mounts(target_host, &config)?;
    }
    Ok(())
}
