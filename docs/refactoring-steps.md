# Refactoring Implementation Steps

## Overview

This document provides concrete steps to refactor the Halvor codebase from monolithic structure to modular workspace.

## Pre-requisites

- [ ] All current changes committed to git
- [ ] All tests passing
- [ ] Create a refactoring branch: `git checkout -b refactor/modular-architecture`

## Phase 1: Database Extraction (Day 1-2)

### Step 1.1: Create Database Crate

```bash
mkdir -p crates/halvor-db/src
touch crates/halvor-db/Cargo.toml
```

**crates/halvor-db/Cargo.toml**:
```toml
[package]
name = "halvor-db"
version = "0.1.0"
edition = "2021"

[dependencies]
rusqlite = { version = "0.38", features = ["bundled"] }
anyhow = "1.0"
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.10", features = ["v4"] }
```

### Step 1.2: Move Database Code

```bash
# Copy (don't move yet - keep backup)
cp -r projects/core/db/* crates/halvor-db/src/

# Structure should be:
# crates/halvor-db/src/
#   ├── lib.rs (create new)
#   ├── core/
#   ├── generated/
#   ├── migrations/
#   ├── migrate.rs
#   └── mod.rs (if needed)
```

**crates/halvor-db/src/lib.rs**:
```rust
pub mod core;
pub mod generated;
pub mod migrate;
pub mod migrations;

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

const DB_FILE_NAME: &str = "halvor.db";

/// Get the database file path
pub fn get_db_path() -> Result<PathBuf> {
    // This will need to import from halvor-core eventually
    // For now, hardcode or make it configurable
    let home = std::env::var("HOME")?;
    Ok(PathBuf::from(home).join(".config/halvor").join(DB_FILE_NAME))
}

/// Get a database connection
pub fn get_connection() -> Result<Connection> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)?;
    migrations::run_migrations(&conn)?;
    Ok(conn)
}

// Re-export for convenience
pub use generated::*;
```

### Step 1.3: Add to Workspace

**Cargo.toml** (root):
```toml
[workspace]
members = [
    "projects/core",
    "crates/halvor-db",
]

[workspace.dependencies]
anyhow = "1.0"
rusqlite = { version = "0.38", features = ["bundled"] }
chrono = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Step 1.4: Update Core to Use halvor-db

**projects/core/Cargo.toml**:
```toml
[dependencies]
halvor-db = { path = "../../crates/halvor-db" }
# Remove rusqlite if only used through halvor-db
```

**Find and replace in projects/core**:
```bash
# Find all db imports
grep -r "crate::db::" projects/core/

# Replace with:
# crate::db:: → halvor_db::
```

### Step 1.5: Test

```bash
cargo test -p halvor-db
cargo test -p halvor  # Make sure core still works
```

## Phase 2: Core Restructuring (Day 3-4)

### Step 2.1: Create Core Crate

```bash
mkdir -p crates/halvor-core
mv projects/core crates/halvor-core-old
```

### Step 2.2: Setup Core Package

**crates/halvor-core/Cargo.toml**:
```toml
[package]
name = "halvor-core"
version = "0.0.6"
edition = "2021"

[dependencies]
halvor-db = { path = "../halvor-db" }

# Copy most dependencies from old Cargo.toml
# Exclude: clap (that's for CLI), axum (that's for agent)
anyhow.workspace = true
tokio = { version = "1", features = ["full"] }
serde.workspace = true
serde_json.workspace = true
# ... etc
```

### Step 2.3: Move Core Code

```bash
mkdir -p crates/halvor-core/src
cp -r crates/halvor-core-old/src/config crates/halvor-core/src/
cp -r crates/halvor-core-old/src/services crates/halvor-core/src/
cp -r crates/halvor-core-old/src/utils crates/halvor-core/src/
# Don't copy: commands/, agent/, db/ (those move to other crates)
```

**crates/halvor-core/src/lib.rs**:
```rust
pub mod config;
pub mod services;
pub mod utils;

// Re-export commonly used items
pub use config::ConfigManager;
```

### Step 2.4: Test Core

```bash
cargo test -p halvor-core
```

## Phase 3: Agent Extraction (Day 5-6)

### Step 3.1: Create Agent Crate

```bash
mkdir -p crates/halvor-agent/src
```

**crates/halvor-agent/Cargo.toml**:
```toml
[package]
name = "halvor-agent"
version = "0.0.6"
edition = "2021"

[lib]
name = "halvor_agent"
path = "src/lib.rs"

[[bin]]
name = "halvor-agent"
path = "src/main.rs"

[dependencies]
halvor-core = { path = "../halvor-core" }
halvor-db = { path = "../halvor-db" }

anyhow.workspace = true
tokio.workspace = true
axum = "0.8"
serde.workspace = true
serde_json.workspace = true
uuid = { version = "1.10", features = ["v4"] }
base64 = "0.22"
chrono = "0.4"

[features]
default = []
web = ["halvor-web"]  # Optional web UI

[dependencies.halvor-web]
path = "../halvor-web"
optional = true
```

### Step 3.2: Create Library Interface

**crates/halvor-agent/src/lib.rs**:
```rust
pub mod server;
pub mod mesh;
pub mod mesh_protocol;
pub mod discovery;
pub mod sync;
pub mod api;

pub use server::AgentServer;

use anyhow::Result;

/// Start the agent server
///
/// # Arguments
/// * `port` - Agent API port (default: 13500)
/// * `web_port` - Optional web UI port (enables UI if Some)
pub async fn start(port: u16, web_port: Option<u16>) -> Result<()> {
    let server = AgentServer::new(port, web_port);
    server.start().await
}
```

### Step 3.3: Create Standalone Binary

**crates/halvor-agent/src/main.rs**:
```rust
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "halvor-agent")]
#[command(about = "Halvor Agent Server")]
struct Args {
    /// Port for agent API
    #[arg(long, default_value = "13500")]
    port: u16,

    /// Port for web UI (enables UI if provided)
    #[arg(long)]
    web_port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting Halvor Agent");
    println!("  Agent API: http://0.0.0.0:{}", args.port);
    if let Some(web_port) = args.web_port {
        println!("  Web UI: http://0.0.0.0:{}", web_port);
    }

    halvor_agent::start(args.port, args.web_port).await
}
```

### Step 3.4: Move Agent Code

```bash
cp -r crates/halvor-core-old/src/agent/* crates/halvor-agent/src/
# Update imports: crate::agent:: → crate::
# Update imports: crate::config:: → halvor_core::config::
# Update imports: crate::db:: → halvor_db::
```

### Step 3.5: Test Agent

```bash
# Test as library
cargo test -p halvor-agent

# Test as standalone binary
cargo run -p halvor-agent -- --port 13500 --web-port 3000
```

## Phase 4: CLI Creation (Day 7-8)

### Step 4.1: Create CLI Crate

```bash
mkdir -p crates/halvor-cli/src/commands
```

**crates/halvor-cli/Cargo.toml**:
```toml
[package]
name = "halvor-cli"
version = "0.0.6"
edition = "2021"

[[bin]]
name = "halvor"
path = "src/main.rs"

[dependencies]
halvor-core = { path = "../halvor-core" }
halvor-db = { path = "../halvor-db" }
halvor-agent = { path = "../halvor-agent" }

clap = { version = "4.5", features = ["derive"] }
anyhow.workspace = true
tokio.workspace = true
serde.workspace = true
```

### Step 4.2: Create Main Entry Point

**crates/halvor-cli/src/main.rs**:
```rust
use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;

#[derive(Parser)]
#[command(name = "halvor")]
#[command(about = "Halvor - Homelab Automation Layer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage halvor agent
    Agent {
        #[command(subcommand)]
        command: commands::agent::AgentCommands,
    },
    /// Show system status
    Status {
        #[command(subcommand)]
        command: Option<commands::status::StatusCommands>,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        command: commands::config::ConfigCommands,
    },
    /// Install services
    Install {
        service: String,
    },
    // ... etc
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Agent { command } => {
            commands::agent::handle(command).await
        }
        Commands::Status { command } => {
            commands::status::handle(command).await
        }
        Commands::Config { command } => {
            commands::config::handle(command).await
        }
        Commands::Install { service } => {
            commands::install::handle(&service).await
        }
    }
}
```

### Step 4.3: Create Agent Command (Embedded)

**crates/halvor-cli/src/commands/agent.rs**:
```rust
use clap::Subcommand;
use anyhow::Result;

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Start the agent
    Start {
        /// Port for agent API
        #[arg(long, default_value = "13500")]
        port: u16,

        /// Enable web UI
        #[arg(long)]
        ui: bool,

        /// Port for web UI (only with --ui)
        #[arg(long, default_value = "3000")]
        web_port: u16,

        /// Run as daemon
        #[arg(long)]
        daemon: bool,
    },
    /// Stop the agent
    Stop,
    /// Agent status
    Status,
    /// Generate join token
    Token,
    /// Join mesh
    Join {
        token: Option<String>,
    },
    /// List peers
    Peers,
    /// Sync with mesh
    Sync,
}

pub async fn handle(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::Start { port, ui, web_port, daemon } => {
            if daemon {
                // Fork to background
                start_daemon(port, ui, web_port).await
            } else {
                // Run in foreground (embed agent library)
                let web = if ui { Some(web_port) } else { None };
                halvor_agent::start(port, web).await
            }
        }
        AgentCommands::Stop => {
            // Stop daemon
            todo!()
        }
        AgentCommands::Token => {
            // Generate token
            todo!()
        }
        // ... etc
    }
}

async fn start_daemon(port: u16, ui: bool, web_port: u16) -> Result<()> {
    #[cfg(unix)]
    {
        use std::process::Command;

        // Spawn halvor-agent binary if available, or embed
        if which::which("halvor-agent").is_ok() {
            Command::new("halvor-agent")
                .arg("--port").arg(port.to_string())
                .args(if ui {
                    vec!["--web-port", &web_port.to_string()]
                } else {
                    vec![]
                })
                .spawn()?;
        } else {
            // Fallback: spawn self in background
            // This is for systems without separate halvor-agent binary
            todo!("Implement daemon mode")
        }
    }

    Ok(())
}
```

### Step 4.4: Move Commands

```bash
cp -r crates/halvor-core-old/src/commands/* crates/halvor-cli/src/commands/
# Exclude: build.rs, dev.rs (those go to separate crates)
# Update imports appropriately
```

### Step 4.5: Test CLI

```bash
# Build CLI
cargo build -p halvor-cli

# Test embedded agent
cargo run -p halvor-cli -- agent start --ui --web-port 3000
```

## Phase 5: Build/Dev Tools (Day 9-10)

### Step 5.1: Extract Build Tool

```bash
mkdir -p crates/halvor-build/src
```

**crates/halvor-build/Cargo.toml**:
```toml
[package]
name = "halvor-build"
version = "0.0.6"
edition = "2021"

[[bin]]
name = "halvor-build"
path = "src/main.rs"

[dependencies]
halvor-core = { path = "../halvor-core" }
clap = { version = "4.5", features = ["derive"] }
anyhow.workspace = true
```

Move build commands from old codebase.

### Step 5.2: Extract Dev Tool

Similar to build tool, but for dev commands.

## Phase 6: FFI Separation (Day 11-12)

### Step 6.1: Create FFI Project

```bash
mkdir -p projects/ffi/src/bindings
```

**projects/ffi/Cargo.toml** (NOT in workspace):
```toml
[package]
name = "halvor-ffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
# Reference main crates (published or path)
halvor-core = { path = "../../crates/halvor-core" }
halvor-db = { path = "../../crates/halvor-db" }

uniffi = "0.28"
wasm-bindgen = "0.2"
```

### Step 6.2: Curate Exports

**projects/ffi/src/exports.rs**:
```rust
// Only export safe, curated functions

#[uniffi::export]
pub fn get_tailscale_status() -> Result<String> {
    halvor_core::services::tailscale::get_status()
}

// etc - manually choose what to expose
```

## Phase 7: Cleanup (Day 13-14)

### Step 7.1: Remove Old Code

```bash
rm -rf crates/halvor-core-old
rm -rf projects/core
```

### Step 7.2: Update Workspace

**Cargo.toml** (final):
```toml
[workspace]
members = [
    "crates/halvor-core",
    "crates/halvor-db",
    "crates/halvor-agent",
    "crates/halvor-cli",
    "crates/halvor-build",
    "crates/halvor-dev",
]

# projects/ffi is separate (not in workspace)
```

### Step 7.3: Update Build Scripts

**Makefile**:
```makefile
install-cli:
    cargo build --release --bin halvor
    cp target/release/halvor ~/.cargo/bin/

install-agent:
    cargo build --release --bin halvor-agent
    cp target/release/halvor-agent ~/.cargo/bin/

dev:
    cargo run -p halvor-dev

build:
    cargo run -p halvor-build
```

### Step 7.4: Update Documentation

- [ ] Update README.md
- [ ] Update CLAUDE.md
- [ ] Update all docs references
- [ ] Add architecture diagram

## Testing Checklist

After each phase:

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` has no errors
- [ ] All binaries run: `halvor`, `halvor-agent`, `halvor-build`, `halvor-dev`
- [ ] Agent embeds in CLI correctly
- [ ] Agent runs standalone in Docker
- [ ] FFI builds and generates bindings

## Rollback Plan

If issues arise:

1. Keep old code in `crates/halvor-core-old` until fully migrated
2. Each phase is independent - can rollback individual phases
3. Git branch protection - don't merge until all tests pass

## Success Criteria

- [ ] Binary sizes reduced (CLI < 15MB)
- [ ] Clear module boundaries
- [ ] All tests passing
- [ ] Documentation updated
- [ ] CI/CD pipelines working
- [ ] Docker builds successfully
- [ ] Mobile apps build with new FFI

## Timeline

- **Week 1**: Phases 1-3 (DB, Core, Agent)
- **Week 2**: Phases 4-7 (CLI, Tools, FFI, Cleanup)
- **Week 3**: Testing, documentation, refinement

Ready to begin!
