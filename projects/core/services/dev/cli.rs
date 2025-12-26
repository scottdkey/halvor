// CLI development mode with watch
use crate::services::build::common::execute_command;
use anyhow::{Context, Result};
use std::process::Command;

/// Start CLI in development mode with watch
pub async fn dev_cli() -> Result<()> {
    println!("Starting CLI in development mode with watch...");

    // Check if cargo-watch is installed
    let watch_available = Command::new("cargo")
        .args(["watch", "--version"])
        .output()
        .is_ok();

    if !watch_available {
        println!("⚠️  cargo-watch not found. Installing...");
        let mut install_cmd = Command::new("cargo");
        install_cmd.args(["install", "cargo-watch"]);
        execute_command(
            install_cmd,
            "Failed to install cargo-watch. Please install manually: cargo install cargo-watch",
        )?;
        println!("✓ cargo-watch installed");
    }

    // Run cargo watch to rebuild on changes
    println!("🔄 Watching for changes... (Press Ctrl+C to stop)");
    println!("💡 The CLI will automatically rebuild when you make changes");
    println!("📦 Building to: target/release/halvor");

    // Note: cargo watch will run until interrupted (Ctrl+C)
    // We use spawn() and wait so it runs in the foreground
    let mut child = Command::new("cargo")
        .args([
            "watch",
            "-x",
            "build --release --bin halvor --manifest-path projects/core/Cargo.toml",
        ])
        .spawn()
        .context("Failed to start cargo watch. Make sure cargo-watch is installed: cargo install cargo-watch")?;

    // Wait for the process to finish (will be interrupted by user with Ctrl+C)
    match child.wait() {
        Ok(status) => {
            if !status.success() {
                eprintln!("⚠️  cargo watch exited with non-zero status");
            }
        }
        Err(e) => {
            eprintln!("⚠️  Error waiting for cargo watch: {}", e);
        }
    }

    Ok(())
}
