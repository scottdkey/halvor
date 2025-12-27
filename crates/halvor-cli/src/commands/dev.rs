// Development command handler - delegates to dev service modules
use halvor_build::{
    dev_cli, dev_ios, dev_mac, dev_web_bare_metal, dev_web_docker, dev_web_prod,
};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Clone)]
pub enum DevCommands {
    /// macOS development mode
    Mac,
    /// iOS development mode
    Ios,
    /// Web development mode
    Web {
        /// Run in bare metal mode (Rust server + Svelte dev, no Docker)
        #[arg(long)]
        bare_metal: bool,
        /// Run in production mode (Docker container)
        #[arg(long)]
        prod: bool,
        /// Run production version locally (uses production Docker container)
        #[arg(long)]
        release: bool,
        /// Port for the web server
        #[arg(long, default_value = "3000")]
        port: u16,
        /// Directory containing built Svelte app (for production mode)
        #[arg(long)]
        static_dir: Option<PathBuf>,
    },
    /// CLI development mode (with watch)
    Cli,
}

pub async fn handle_dev(command: DevCommands) -> Result<()> {
    match command {
        DevCommands::Mac => {
            dev_mac()?;
        }
        DevCommands::Ios => {
            dev_ios()?;
        }
        DevCommands::Web {
            bare_metal,
            prod,
            release,
            port,
            static_dir,
        } => {
            if release {
                // Run production version locally
                dev_web_prod().await?;
            } else if prod {
                dev_web_prod().await?;
            } else if bare_metal {
                dev_web_bare_metal(port, static_dir).await?;
            } else {
                dev_web_docker(port).await?;
            }
        }
        DevCommands::Cli => {
            dev_cli().await?;
        }
    }

    Ok(())
}
