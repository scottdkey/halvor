use crate::config;
use crate::services::pia_vpn as vpn;
use anyhow::Result;

#[derive(clap::Subcommand, Clone)]
pub enum VpnCommands {
    /// Build and push VPN container image to GitHub Container Registry
    Build {
        /// GitHub username or organization
        #[arg(long)]
        github_user: String,
        /// Image tag (if not provided, pushes both 'latest' and git hash)
        #[arg(long)]
        tag: Option<String>,
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
        VpnCommands::Build { github_user, tag } => {
            let build_hostname = "localhost";
            vpn::build_and_push_vpn_image(build_hostname, &github_user, tag.as_deref(), &config)?;
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
