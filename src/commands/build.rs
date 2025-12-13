// Build command handler - delegates to build service modules
use crate::services::build::{
    build_and_sign_ios, build_and_sign_mac, build_android, build_cli, build_web, build_web_docker,
    push_ios_to_app_store, run_web_prod, sign_android,
};
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum BuildCommands {
    /// Build iOS app (always signed)
    Ios {
        /// Push to App Store Connect after building
        #[arg(long)]
        push: bool,
    },
    /// Build macOS app (always signed)
    Mac,
    /// Build Android app (always signed)
    Android,
    /// Build Web app (Rust server + Svelte frontend)
    Web {
        /// Build for production release
        #[arg(long)]
        release: bool,
        /// Build for bare metal (local Rust binary, no Docker)
        #[arg(long)]
        bare_metal: bool,
        /// Run the container after building
        #[arg(long)]
        run: bool,
        /// Build Docker container
        #[arg(long)]
        docker: bool,
        /// Push Docker image to GitHub Container Registry
        #[arg(long)]
        push: bool,
    },
    /// Build CLI binary
    Cli {
        /// Platforms to build for (comma-separated: apple,windows,linux). If not specified, builds all.
        #[arg(long, conflicts_with = "targets")]
        platforms: Option<String>,
        /// Specific targets to build for (comma-separated Rust target triples, e.g., x86_64-unknown-linux-gnu,aarch64-apple-darwin)
        #[arg(long, conflicts_with = "platforms")]
        targets: Option<String>,
        /// Push built binaries to GitHub releases
        #[arg(long)]
        push: bool,
    },
}

pub fn handle_build(command: BuildCommands) -> Result<()> {
    match command {
        BuildCommands::Ios { push } => {
            build_and_sign_ios()?;
            println!("✓ iOS build complete");

            if push {
                push_ios_to_app_store()?;
            }
        }
        BuildCommands::Mac => {
            build_and_sign_mac()?;
            println!("✓ macOS build complete");
        }
        BuildCommands::Android => {
            build_android()?;
            sign_android()?;
            println!("✓ Android build complete");
        }
        BuildCommands::Web {
            release,
            bare_metal,
            run,
            docker,
            push,
        } => {
            // If --bare-metal is specified, build local Rust binary only
            if bare_metal {
                build_web(release)?;
            }
            // If --release is specified, build Docker production container
            // Otherwise, if --docker is specified, build Docker container
            else if release || docker {
                build_web_docker(release, push)?;
            } else {
                build_web(release)?;
                if run {
                    run_web_prod()?;
                }
            }
            println!("✓ Web build complete");
        }
        BuildCommands::Cli { platforms, targets, push } => {
            let platforms_str: Option<&str> = platforms.as_deref();
            let targets_str: Option<&str> = targets.as_deref();
            build_cli(platforms_str, targets_str, push)?;
            println!("✓ CLI build complete");
        }
    }

    Ok(())
}
