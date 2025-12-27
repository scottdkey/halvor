//! Halvor Build Binary
//! CLI entry point for build and dev operations

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "halvor-build")]
#[command(about = "Halvor Build - Build and development operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build operations
    Build {
        #[command(subcommand)]
        subcommand: BuildCommands,
    },
    /// Development operations
    Dev {
        #[command(subcommand)]
        subcommand: DevCommands,
    },
}

#[derive(Subcommand)]
enum BuildCommands {
    /// Build CLI binary
    Cli,
    /// Build iOS app
    Ios,
    /// Build macOS app
    Mac,
    /// Build Android library and app
    Android,
    /// Build web application
    Web,
}

#[derive(Subcommand)]
enum DevCommands {
    /// CLI development mode with watch
    Cli,
    /// macOS development with hot reload
    Mac,
    /// iOS development with simulator
    Ios,
    /// Web development (Docker)
    Web,
    /// Web development (Rust server + Svelte dev)
    #[command(name = "web-bare-metal")]
    WebBareMetal,
    /// Web production mode (Docker)
    #[command(name = "web-prod")]
    WebProd,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { subcommand } => {
            match subcommand {
                BuildCommands::Cli => {
                    halvor_build::build_cli()?;
                }
                BuildCommands::Ios => {
                    halvor_build::build_and_sign_ios()?;
                }
                BuildCommands::Mac => {
                    halvor_build::build_and_sign_mac()?;
                }
                BuildCommands::Android => {
                    halvor_build::build_android()?;
                }
                BuildCommands::Web => {
                    halvor_build::build_web(true)?;
                }
            }
        }
        Commands::Dev { subcommand } => {
            match subcommand {
                DevCommands::Cli => {
                    halvor_build::dev_cli().await?;
                }
                DevCommands::Mac => {
                    halvor_build::dev_mac().await?;
                }
                DevCommands::Ios => {
                    halvor_build::dev_ios().await?;
                }
                DevCommands::Web => {
                    halvor_build::dev_web_docker(8080).await?;
                }
                DevCommands::WebBareMetal => {
                    halvor_build::dev_web_bare_metal(8080, None).await?;
                }
                DevCommands::WebProd => {
                    halvor_build::dev_web_prod().await?;
                }
            }
        }
    }

    Ok(())
}

