use anyhow::Result;

/// Handle backup command
/// hostname: None = local, Some(hostname) = remote host
/// TODO: Implement backup functionality in halvor-agent
pub fn handle_backup(
    _hostname: Option<&str>,
    _service: Option<&str>,
    _env: bool,
    _list: bool,
) -> Result<()> {
    anyhow::bail!("Backup functionality not yet implemented in halvor-agent. This will be added in a future update.");
}
