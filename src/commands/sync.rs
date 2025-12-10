use crate::config;
use crate::services::sync;
use anyhow::Result;

/// Handle sync command
/// hostname: None = local sync (push to remote), Some(hostname) = remote sync (pull from remote)
pub fn handle_sync(hostname: Option<&str>, pull: bool) -> Result<()> {
    let config = config::load_config()?;

    if let Some(hostname) = hostname {
        // Remote sync: sync with specified host
        // If pull=true, we're pulling from that host
        // If pull=false, we're pushing to that host
        sync::sync_data(hostname, pull, &config)?;
    } else {
        // Local sync: push to all configured hosts (or pull from all)
        // For now, this requires a hostname - we could enhance this later
        anyhow::bail!("Sync requires a hostname. Use: halvor <hostname> sync [--pull]");
    }

    Ok(())
}
