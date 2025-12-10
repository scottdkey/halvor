use crate::config;
use crate::services::smb;
use anyhow::Result;

/// Handle SMB command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_smb(hostname: Option<&str>, uninstall: bool) -> Result<()> {
    let config = config::load_config()?;

    // Ensure host is in config, prompt to set up if not
    let target_host = if let Some(host) = hostname {
        // Explicit hostname provided - try to find it (with TLD normalization)
        config::service::ensure_host_in_config(Some(host), &config)?
    } else {
        // No hostname - ensure current machine is set up
        config::service::ensure_host_in_config(None, &config)?
    };

    if uninstall {
        smb::uninstall_smb_mounts(&target_host, &config)?;
    } else {
        smb::setup_smb_mounts(&target_host, &config)?;
    }
    Ok(())
}
