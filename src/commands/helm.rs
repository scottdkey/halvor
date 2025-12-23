//! Helm chart management commands

use crate::config;
use crate::services::helm;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum HelmCommands {
    /// Install a Helm chart
    Install {
        /// Chart name (e.g., radarr, sonarr, pia-vpn)
        chart: String,
        /// Release name (defaults to chart name)
        #[arg(long)]
        name: Option<String>,
        /// Namespace to install into (default: default)
        #[arg(long, short = 'n')]
        namespace: Option<String>,
        /// Values file override
        #[arg(long, short = 'f')]
        values: Option<String>,
        /// Set individual values (key=value)
        #[arg(long)]
        set: Vec<String>,
    },
    /// Upgrade a Helm release
    Upgrade {
        /// Release name
        release: String,
        /// Values file override
        #[arg(long, short = 'f')]
        values: Option<String>,
        /// Set individual values (key=value)
        #[arg(long)]
        set: Vec<String>,
    },
    /// Uninstall a Helm release
    Uninstall {
        /// Release name
        release: String,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// List installed Helm releases
    List {
        /// Show releases in all namespaces
        #[arg(long, short = 'A')]
        all_namespaces: bool,
        /// Filter by namespace
        #[arg(long, short = 'n')]
        namespace: Option<String>,
    },
    /// Show available charts in the halvor repo
    Charts,
    /// Export values from a running release
    ExportValues {
        /// Release name
        release: String,
        /// Output path (default: stdout)
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
}

pub fn handle_helm(hostname: Option<&str>, command: HelmCommands) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    let target_host = hostname.unwrap_or("localhost");

    match command {
        HelmCommands::Install {
            chart,
            name,
            namespace,
            values,
            set,
        } => {
            helm::install_chart(
                target_host,
                &chart,
                name.as_deref(),
                namespace.as_deref(),
                values.as_deref(),
                &set,
                &config,
            )?;
        }
        HelmCommands::Upgrade { release, values, set } => {
            helm::upgrade_release(target_host, &release, values.as_deref(), &set, &config)?;
        }
        HelmCommands::Uninstall { release, yes } => {
            helm::uninstall_release(target_host, &release, yes, &config)?;
        }
        HelmCommands::List {
            all_namespaces,
            namespace,
        } => {
            helm::list_releases(target_host, all_namespaces, namespace.as_deref(), &config)?;
        }
        HelmCommands::Charts => {
            helm::list_charts()?;
        }
        HelmCommands::ExportValues { release, output } => {
            helm::export_values(target_host, &release, output.as_deref(), &config)?;
        }
    }

    Ok(())
}
