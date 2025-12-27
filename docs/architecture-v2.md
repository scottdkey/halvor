# Halvor Architecture v2

## Refined Structure Based on Feedback

### Workspace Layout

```
halvor/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── halvor-core/              # Core business logic
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config/           # Configuration management
│   │   │   ├── services/         # Business logic (docker, ssh, k3s, etc.)
│   │   │   └── utils/            # Utilities (crypto, exec, networking)
│   │
│   ├── halvor-db/                # Database module (separate crate)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── core/             # Database traits and core logic
│   │   │   ├── generated/        # Auto-generated table code
│   │   │   ├── migrations/       # SQL migration files
│   │   │   └── migrate.rs        # Migration runner
│   │
│   ├── halvor-agent/             # Agent server (library + binary)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs            # Library interface (for embedding)
│   │   │   ├── main.rs           # Standalone binary (for Docker)
│   │   │   ├── server.rs         # HTTP/TCP server
│   │   │   ├── mesh.rs           # Mesh protocol
│   │   │   ├── mesh_protocol.rs  # Message definitions
│   │   │   ├── discovery.rs      # Peer discovery
│   │   │   ├── sync.rs           # Data synchronization
│   │   │   └── web.rs            # Web UI server (optional feature)
│   │
│   ├── halvor-cli/               # Main CLI binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/
│   │   │       ├── agent.rs      # Agent commands (embeds halvor-agent)
│   │   │       ├── config.rs     # Config commands
│   │   │       ├── install.rs    # Install services
│   │   │       ├── status.rs     # Status commands
│   │   │       └── ...
│   │
│   ├── halvor-build/             # Build tooling (separate binary)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── cli.rs            # Build CLI
│   │   │   ├── ios.rs            # Build iOS
│   │   │   ├── android.rs        # Build Android
│   │   │   ├── web.rs            # Build web
│   │   │   └── ffi.rs            # Build FFI
│   │
│   └── halvor-dev/               # Dev tooling (separate binary)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── watch.rs          # File watching
│           └── ...
│
├── projects/
│   ├── ffi/                      # SEPARATE PROJECT - not in workspace
│   │   ├── Cargo.toml            # Independent FFI project
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── exports.rs        # Manually curated exports
│   │   │   └── bindings/
│   │   │       ├── swift.rs      # iOS/macOS bindings
│   │   │       ├── kotlin.rs     # Android bindings
│   │   │       └── wasm.rs       # Web bindings
│   │   └── README.md             # FFI-specific documentation
│   │
│   ├── ios/                      # iOS app (Swift)
│   ├── android/                  # Android app (Kotlin)
│   └── web/                      # Web UI (SvelteKit)
```

## Key Decisions

### 1. Database as Separate Crate

**Why**: Database is a distinct concern with its own dependencies and complexity

**Benefits**:
- Can be versioned independently
- Clear migration strategy
- Easier to test in isolation
- Can be shared by core, agent, and CLI

**Dependencies**:
```toml
# halvor-db/Cargo.toml
[dependencies]
rusqlite = "0.38"
anyhow = "1.0"
chrono = "0.4"
```

### 2. Agent: Library + Binary

**Library** (`halvor-agent` as lib):
- Embedded in `halvor-cli` for `halvor agent start`
- Provides `AgentServer` struct with `start()` method
- Optional `web` feature for UI server

**Binary** (`halvor-agent` as bin):
- Standalone for Docker containers
- Minimal wrapper around library
- Same code, different entry point

```rust
// halvor-agent/src/lib.rs
pub struct AgentServer {
    port: u16,
    web_port: Option<u16>,
}

impl AgentServer {
    pub fn new(port: u16, web_port: Option<u16>) -> Self { ... }
    pub async fn start(&self) -> Result<()> { ... }
}

// halvor-agent/src/main.rs
use halvor_agent::AgentServer;

#[tokio::main]
async fn main() -> Result<()> {
    let server = AgentServer::new(13500, Some(3000));
    server.start().await
}
```

### 3. CLI Embeds Agent

**Usage**:
```bash
# With UI
halvor agent start --ui --port 3000

# Without UI
halvor agent start

# Custom ports
halvor agent start --port 13500 --ui --web-port 3000
```

**Implementation**:
```rust
// halvor-cli/src/commands/agent.rs
use halvor_agent::AgentServer;

pub async fn handle_agent_start(port: u16, ui: bool, web_port: Option<u16>) -> Result<()> {
    let web_port = if ui {
        Some(web_port.unwrap_or(3000))
    } else {
        None
    };

    let server = AgentServer::new(port, web_port);
    server.start().await
}
```

### 4. FFI as Separate Project

**Why separate**:
- Different versioning and release cycle
- Explicit control over exposed APIs
- Smaller surface area for mobile/web
- Can update without rebuilding main project

**Structure**:
```
projects/ffi/
├── Cargo.toml              # References halvor-core, halvor-db as dependencies
├── src/
│   ├── lib.rs
│   ├── exports.rs          # Manually curated API surface
│   └── bindings/
│       ├── swift.rs        # UniFFI for Swift
│       ├── kotlin.rs       # UniFFI for Kotlin
│       └── wasm.rs         # wasm-bindgen for Web
```

**Exports** (curated, not everything from core):
```rust
// projects/ffi/src/exports.rs

use halvor_core::services;
use halvor_db;

/// Get Tailscale status (safe to expose)
#[uniffi::export]
pub fn get_tailscale_status() -> Result<TailscaleStatus> {
    services::tailscale::get_status()
}

/// Get K3s status (safe to expose)
#[uniffi::export]
pub fn get_k3s_status() -> Result<K3sStatus> {
    services::k3s::get_status()
}

// NOT exported:
// - Database migration functions
// - SSH key management
// - Build tooling
// - Development utilities
```

## Module Boundaries

### halvor-core
**Purpose**: Shared business logic
**Size**: ~5MB
**Used by**: CLI, Agent, FFI
**Contains**: Services, config, utils (no DB, no commands)

### halvor-db
**Purpose**: Database layer
**Size**: ~2MB
**Used by**: Core, Agent, CLI
**Contains**: Schema, migrations, generated code, traits

### halvor-agent (lib + bin)
**Purpose**: Mesh server
**Size**: ~12MB (bin), ~8MB (lib)
**Used by**: CLI (as lib), Docker (as bin)
**Contains**: Server, mesh protocol, discovery, sync, web UI

### halvor-cli
**Purpose**: User-facing CLI
**Size**: ~15MB (includes embedded agent)
**Distribution**: Public release
**Contains**: Runtime commands, embeds agent library

### halvor-build
**Purpose**: Build tooling
**Size**: ~80MB
**Distribution**: Development only
**Contains**: Cross-compilation, packaging, FFI generation

### halvor-dev
**Purpose**: Development mode
**Size**: ~80MB
**Distribution**: Development only
**Contains**: Hot reload, watch mode, reuses build logic

### projects/ffi
**Purpose**: Mobile/Web bindings
**Size**: ~15MB
**Distribution**: Build artifact (not distributed directly)
**Contains**: Curated API exports for iOS/Android/Web

## Command Examples

```bash
# CLI - Agent with UI (embedded)
halvor agent start --ui --port 3000
# Runs: Agent on :13500, Web UI on :3000, same process

halvor agent start --ui
# Runs: Agent on :13500, Web UI on :3000 (default), same process

halvor agent start
# Runs: Agent on :13500, no UI

# Docker - Standalone agent binary
docker run halvor-agent --port 13500 --web-port 3000
# Uses halvor-agent binary (main.rs)

# Docker Compose
services:
  agent:
    image: halvor-agent
    command: ["--port", "13500", "--web-port", "3000"]
    ports:
      - "13500:13500"  # Agent API
      - "3000:3000"    # Web UI
```

## Build Process

### For Users (Release)
```bash
cargo build --release --bin halvor-cli
# Output: target/release/halvor (~15MB)
# Includes: Core, DB, Agent (embedded), CLI commands
```

### For Docker
```bash
cargo build --release --bin halvor-agent
# Output: target/release/halvor-agent (~12MB)
# Includes: Core, DB, Agent, Web server
```

### For Development
```bash
cargo build --bin halvor-build
cargo build --bin halvor-dev
# Development tools (not distributed to users)
```

### For Mobile/Web
```bash
cd projects/ffi
cargo build --release
# Generates bindings for iOS, Android, Web
# Only exposes curated API from exports.rs
```

## Cargo Workspace

```toml
# /Cargo.toml (workspace root)
[workspace]
members = [
    "crates/halvor-core",
    "crates/halvor-db",
    "crates/halvor-agent",
    "crates/halvor-cli",
    "crates/halvor-build",
    "crates/halvor-dev",
]

# projects/ffi is NOT in workspace (separate project)

[workspace.dependencies]
# Shared dependencies
anyhow = "1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ... etc
```

## Dependency Graph

```
┌─────────────┐     ┌─────────────┐
│ halvor-core │ ←── │  halvor-db  │
└─────────────┘     └─────────────┘
       ↑                   ↑
       │                   │
       ├───────────────────┴────────────┐
       │                                │
┌─────────────┐                  ┌─────────────┐
│halvor-agent │                  │ halvor-cli  │
│  (lib+bin)  │ ←──embedded──────│   (bin)     │
└─────────────┘                  └─────────────┘
       ↑
       │
┌─────────────┐
│projects/ffi │  (separate project)
│ (uncurated - direct export of all halvor functionality)   │
└─────────────┘
```

## Migration Steps

### Phase 1: Database Extraction
1. Create `crates/halvor-db/`
2. Move `projects/core/db/` → `crates/halvor-db/src/`
3. Update all imports: `crate::db::` → `halvor_db::`
4. Test: `cargo test -p halvor-db`

### Phase 2: Agent Extraction
1. Create `crates/halvor-agent/`
2. Move agent code from `projects/core/agent/`
3. Create both `lib.rs` (for embedding) and `main.rs` (for Docker)
4. Add `web` feature flag for UI server
5. Test embedded: `cargo test -p halvor-cli`
6. Test standalone: `cargo run -p halvor-agent`

### Phase 3: CLI Cleanup
1. Move `projects/core` → `crates/halvor-core`
2. Create `crates/halvor-cli/`
3. Move command handlers to CLI
4. Embed agent library
5. Remove build/dev commands
6. Test: `cargo run -p halvor-cli agent start --ui`

### Phase 4: Build/Dev Tools
1. Extract build commands to `crates/halvor-build/`
2. Extract dev commands to `crates/halvor-dev/`
3. Update Makefile to use new binaries

### Phase 5: FFI Separation
1. Create `projects/ffi/` (outside workspace)
2. Add halvor-core, halvor-db as dependencies
3. Manually curate exports
4. Generate bindings
5. Update iOS/Android/Web projects

## Benefits of This Architecture

1. **Single binary for users**: `halvor` (~15MB) includes everything needed
2. **Flexible deployment**: Agent can run embedded or standalone
3. **Small Docker images**: Only `halvor-agent` binary needed (~12MB)
4. **Controlled API surface**: FFI only exposes curated functions
5. **Clear boundaries**: Each crate has one purpose
6. **Dev tools separate**: Don't bloat user releases
7. **Database isolation**: Can version/test DB independently

## Questions Resolved

✅ **Agent embedded or separate?** Both! Library for embedding, binary for Docker
✅ **Command structure?** `halvor` is main CLI, build/dev are separate binaries
✅ **FFI placement?** Separate project with curated exports
✅ **Web UI flag?** `--ui` flag to enable, `--web-port` to configure
✅ **Database?** Separate crate, shared by all

Ready to implement!
