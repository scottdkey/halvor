//! Provision Command
//!
//! Guided setup for new hosts in the halvor ecosystem.
//!
//! Usage:
//!   halvor provision              # Interactive guided setup on localhost
//!   halvor provision -H <host>    # Interactive guided setup on remote host
//!   halvor provision -y           # Non-interactive with defaults

use crate::config;
use crate::services::provision;
use anyhow::Result;

/// Handle provision command
pub fn handle_provision(
    hostname: Option<&str>,
    skip_prompts: bool,
    portainer_host: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    if skip_prompts {
        // Non-interactive mode - just install defaults
        provision::provision_defaults(target_host, portainer_host, &config)?;
    } else {
        // Interactive guided mode
        provision::provision_guided(target_host, &config)?;
    }

    Ok(())
}
