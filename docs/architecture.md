# Architecture Guide

This document describes the architecture of halvor, including the multi-platform FFI system and agent mesh network.

## System Architecture

```
halvor (Rust core)
    ├── CLI Binary (halvor)
    ├── Library Crate (halvor)
    ├── FFI Layer
    │   ├── C FFI (Swift/iOS/macOS)
    │   ├── JNI (Android/Kotlin)
    │   └── WASM (Web/TypeScript)
    └── Services
        ├── Docker
        ├── K3s
        ├── Helm
        ├── SMB
        ├── Tailscale
        └── Agent
```

## Core Structure

### Commands Layer

Commands (`src/commands/`) are CLI entry points that delegate to Services (`src/services/`). This separation enables FFI exports - services can be called from Swift, Kotlin, or WASM without CLI dependencies.

```
User invokes CLI → main.rs → commands/*.rs → services/*.rs → utils/*.rs
                                            ↓
                                      FFI bindings (ffi/*.rs)
                                            ↓
                      Swift/Kotlin/WASM wrappers call services directly
```

### Key Modules

- **`src/commands/`** - CLI command handlers (agent, backup, build, config, dev, install, init, join, status, sync, uninstall, update)
- **`src/services/`** - Business logic implementations (platform-agnostic, exported via FFI)
- **`src/config/`** - Configuration management using TOML files and environment variables
- **`src/db/`** - SQLite database abstraction with migrations
- **`src/agent/`** - HTTP server (Axum) for agent daemon mode
- **`src/ffi/`** - Foreign Function Interface layer
- **`src/utils/`** - Cross-cutting utilities (exec, ssh, crypto, networking)

## Multi-Platform FFI System

The multi-platform FFI system generates bindings for Swift (iOS/macOS), Kotlin (Android), and TypeScript (Web/WASM).

### Architecture

```
halvor (Rust core)
    ├── halvor-ffi (C FFI for Swift)
    ├── halvor-ffi-wasm (WASM for Web)
    ├── halvor-ffi-jni (JNI for Android)
    └── halvor-ffi-macro (Code generation macros)
```

### Macros

#### `#[multi_platform_export]`

Marks a function for all platforms. Equivalent to using all three platform-specific macros.

```rust
use halvor_ffi_macro::multi_platform_export;

#[multi_platform_export]
pub fn discover_agents(client: &HalvorClient) -> Result<Vec<DiscoveredHost>, String> {
    client.discover_agents()
}
```

This single annotation makes the function available in:
- **Swift**: `try client.discoverAgents()`
- **Kotlin**: `client.discoverAgents()`
- **TypeScript**: `await wasmModule.discoverAgents()`

### Build Process

1. **Rust Compilation**: Functions are compiled with platform-specific targets
2. **Code Generation**: Build scripts generate bindings for each platform
3. **Integration**: Generated code is automatically included in each platform's build

### Platform-Specific Builds

#### CLI Binaries

```bash
# Build for current platform (auto-detects OS)
halvor build cli

# Build for specific platforms
halvor build cli --platforms apple
halvor build cli --platforms linux
halvor build cli --platforms windows
halvor build cli --platforms apple,linux,windows
```

#### Swift (iOS/macOS)

```bash
# Build iOS app
halvor build ios

# Build macOS app
halvor build mac
```

#### Android

```bash
# Build Android library and app
halvor build android
```

#### Web (WASM + SvelteKit)

```bash
# Build web application
halvor build web
```

### Generated Files

- **Swift**: `projects/ios/Sources/HalvorSwiftFFI/halvor_ffi/generated_swift_bindings.swift`
- **Kotlin**: `projects/android/src/main/kotlin/dev/scottkey/halvor/GeneratedBindings.kt`
- **TypeScript**: `projects/web/src/lib/halvor-ffi/generated-bindings.ts`

All generated files are automatically included in their respective build systems.

### Adding FFI-Exported Functions

1. Implement function in service module
2. Annotate with `#[multi_platform_export]`
3. Rebuild platform bindings:
   - Swift: `cd projects/ios && ./build.sh`
   - Android: `halvor build android`
   - Web: `halvor build web`

### Platform Support

- **macOS**: Full support for all platforms (CLI, iOS, macOS, Android, Web)
- **Linux**: CLI and Web development (iOS/macOS builds not available)
- **Windows**: CLI only (via WSL recommended)

## Agent Architecture

Halvor supports a mesh network architecture where each host runs a halvor agent daemon that:

- Automatically discovers other halvor agents on the network
- Syncs configuration data bidirectionally
- Provides secure remote command execution
- Maintains host information (IPs, Tailscale addresses, etc.)

### Architecture Components

#### 1. Halvor Agent Daemon (`halvor agent`)

A background service that runs on each host:

- Listens on a configurable port (default: 13500)
- Maintains secure connections to other agents
- Handles remote command execution requests
- Syncs database/config state with peers
- Auto-discovers hosts via Tailscale/local network

#### 2. Secure Communication

- **TLS/mTLS**: Each agent generates a certificate on first run
- **Shared Secret**: Optional shared secret for additional authentication
- **Token-based**: Short-lived tokens for command execution
- **Fallback to SSH**: If agent unavailable, fall back to existing SSH mechanism

#### 3. Host Discovery

- **Tailscale Integration**: Query Tailscale API to discover other halvor agents
- **Local Network Scan**: Scan local network for halvor agents
- **Manual Registration**: Allow manual host registration
- **Auto-sync**: Automatically sync host info (IP, Tailscale address, hostname)

#### 4. Config Sync Mesh

- **Bidirectional Sync**: Configs sync between all connected agents
- **Conflict Resolution**: Last-write-wins or manual resolution
- **Encrypted Data**: Encrypted env data syncs securely
- **Database Replication**: SQLite database syncs between hosts

#### 5. Command Execution API

- **RPC-style API**: JSON-RPC or gRPC for command execution
- **Streaming Output**: Support for streaming command output
- **File Operations**: Secure file transfer and operations
- **Permission Model**: Role-based access control

### Implementation Plan

#### Phase 1: Core Agent Infrastructure

1. Create `halvor agent` command (start/stop/status)
2. Basic HTTP/HTTPS server for agent communication
3. Simple authentication mechanism
4. Host discovery via Tailscale

#### Phase 2: Secure Communication

1. TLS certificate generation and management
2. mTLS for mutual authentication
3. Token-based session management

#### Phase 3: Config Sync

1. Database sync mechanism
2. Config file sync
3. Conflict resolution

#### Phase 4: Command Execution

1. Remote command execution API
2. Update Executor to use agent API
3. SSH fallback mechanism

#### Phase 5: Mesh Networking

1. Automatic peer discovery
2. Mesh topology management
3. Health checks and reconnection

### Security Considerations

- All communication encrypted (TLS)
- Mutual authentication (mTLS)
- Token expiration and rotation
- Rate limiting on API endpoints
- Audit logging
- Optional shared secret for additional security

### Benefits

1. **Better Security**: TLS/mTLS instead of plain SSH
2. **Automatic Discovery**: No manual host configuration needed
3. **Config Sync**: Changes propagate automatically
4. **Fault Tolerance**: Mesh network provides redundancy
5. **Performance**: Direct connections, no SSH overhead
6. **Scalability**: Easy to add new hosts

## Database Architecture

### SQLite Database

Halvor uses SQLite for persistent storage:

- **Schema**: Defined in `src/db/migrations/`
- **Generated Code**: Table code in `src/db/generated/`
- **Core Traits**: Database operations in `src/db/core/`
- **Migrations**: Run automatically on startup via `db::migrate::run_migrations()`

### Current Schema Version

Version: 004

### Tables

- **host_info**: Host configurations (IP, hostname, backup paths, etc.)
- Additional tables as needed for services

## Configuration Architecture

### Environment Variables

Configuration is loaded from `.env` files with support for:

- **direnv + 1Password**: Secure secret management
- **Manual .env files**: Direct file editing
- **Database storage**: Encrypted storage in SQLite

### Configuration Format

```
HOST_<HOSTNAME>_<FIELD>=<value>
```

Example:
```
HOST_FRIGG_IP="10.10.10.10"
HOST_FRIGG_HOSTNAME="frigg.ts.net"
HOST_FRIGG_BACKUP_PATH="/mnt/backups/frigg"
```

## Service Architecture

### Service Categories

1. **Platform Tools**: Installed natively (docker, tailscale, smb, k3s)
2. **Helm Charts**: Deployed to Kubernetes cluster (portainer, gitea, etc.)

### App Registry

All installable apps are defined in `src/services/apps.rs`:

- **AppDefinition**: Name, category, description, namespace
- **Auto-detection**: CLI automatically detects Helm charts vs platform tools
- **No flags needed**: No `--helm` flag required - detection is automatic

### Installation Flow

1. User runs: `halvor install <app> -H <hostname>`
2. Command looks up app in registry
3. Determines category (Platform vs HelmChart)
4. Routes to appropriate service:
   - Platform → `install_platform_tool()`
   - HelmChart → `helm::install_chart()` (with cluster validation)

## Build System Architecture

### Cross-Compilation

The build system supports full cross-compilation:

- **Native builds**: Uses `cargo build --target <target>`
- **Cross-OS compilation**: Automatically uses `cross` tool (Docker-based)
- **Automatic detection**: Detects when `cross` is needed

### CI/CD Workflows

GitHub Actions workflows in `.github/workflows/`:

- `build-linux.yml` - Multi-arch Linux builds
- `build-macos.yml` - macOS Universal binaries
- `build-windows.yml` - Windows x86_64 and ARM64
- `build-pia-vpn.yml` - Docker container builds

Releases are published to GitHub Releases. Docker images are pushed to GitHub Container Registry (ghcr.io).

