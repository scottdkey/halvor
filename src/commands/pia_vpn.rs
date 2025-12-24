use crate::config;
use crate::services::docker;
use crate::services::pia_vpn as vpn;
use anyhow::Result;

#[derive(clap::Subcommand, Clone)]
pub enum VpnCommands {
    /// Build and push VPN container image to GitHub Container Registry
    Build {
        /// Push image to registry after building
        #[arg(long)]
        push: bool,
        /// Use 'latest' tag instead of 'experimental'
        #[arg(long)]
        release: bool,
        /// Build without using cache
        #[arg(long)]
        no_cache: bool,
    },
    /// Deploy VPN to a remote host (injects PIA credentials from local .env)
    Deploy {
        /// Hostname to deploy VPN to
        hostname: String,
    },
    /// Verify VPN is working correctly
    Verify {
        /// Hostname where VPN is running
        hostname: String,
    },
}

pub fn handle_vpn(command: VpnCommands) -> Result<()> {
    let config = config::load_config()?;

    match command {
        VpnCommands::Build {
            push,
            release,
            no_cache,
        } => {
            // Use centralized docker build system
            docker::build_container("pia-vpn", no_cache, push, release)?;
        }
        VpnCommands::Deploy { hostname } => {
            vpn::deploy_vpn(&hostname, &config)?;
        }
        VpnCommands::Verify { hostname } => {
            vpn::verify_vpn(&hostname, &config)?;
        }
    }

    Ok(())
}
