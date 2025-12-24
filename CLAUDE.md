# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**HAL (Homelab Automation Layer)** is a multi-platform Rust CLI tool for managing homelab infrastructure. The project compiles to native binaries (Linux/macOS/Windows), mobile apps (iOS/Android via FFI), and web applications (WASM). It provides automation for Docker, SSH, VPN, SMB, Tailscale, Portainer, and Nginx Proxy Manager.

## Build Commands (For Reference Only - User Will Execute)

### CLI Development
```bash
# Build and install CLI to system (macOS only by default)
make install-cli
# or
cargo build --release --bin halvor && cargo install --path . --bin halvor --force

# Build for all platforms (requires Docker for Linux/Windows)
halvor build cli --platforms apple,linux,windows

# Development mode with auto-rebuild (uses cargo-watch)
halvor dev cli
# or
make dev

# Run tests
cargo test
```

### Platform-Specific Builds
```bash
# CLI binaries
halvor build cli                    # macOS only (default, no Docker needed)
halvor build cli --platforms apple  # Same as above
halvor build cli --platforms linux  # Linux targets (requires Docker)
halvor build cli --platforms apple,linux,windows  # All platforms (requires Docker)

# iOS app
halvor build ios

# macOS app
halvor build mac

# Android library and app
halvor build android

# Web application (WASM + SvelteKit)
halvor build web
```

### Development Modes with Hot Reload
```bash
# macOS development
halvor dev mac

# iOS simulator
halvor dev ios

# Web (Docker-based)
halvor dev web

# Web (bare-metal: Rust server + Svelte dev)
halvor dev web --bare-metal

# Web production mode
halvor dev web --prod
```

### Installation

**Platform Support:**
- **macOS**: Full support for all platforms (CLI, iOS, macOS, Android, Web)
- **Linux**: CLI and Web development (iOS/macOS builds not available)
- **Windows**: CLI only (via WSL recommended)

Install all dependencies for all platforms:
```bash
make install
```

Install individual platform dependencies:
```bash
make install-rust          # Rust toolchain (all platforms)
make install-rust-targets  # Cross-compilation targets (all platforms)
make install-swift         # Xcode/Swift dependencies (macOS only)
make install-android       # Android NDK/Java (optional on Linux)
make install-web           # Node.js, wasm-pack (all platforms)
make install-tools         # Docker, direnv, 1Password CLI, Fastlane (all platforms)
```

**Linux-Specific Notes:**
- Swift/Xcode dependencies are automatically skipped on Linux (macOS only)
- Ruby/Fastlane are automatically skipped on Linux (macOS only, for iOS/macOS builds)
- Java 17 (OpenJDK) is automatically installed via system package manager
- Node.js 24 LTS is installed via NVM (Node Version Manager) for all platforms
- Cross-compilation toolchains (`gcc-aarch64-linux-gnu`, `musl-tools`) are automatically installed on Debian/Ubuntu
- All Rust-based CLI and Web features work natively on Linux
- Linux can cross-compile to all Linux architectures (x86_64, aarch64, gnu, musl)

### Cross-Compilation Setup

**TL;DR:**
- **Local development**: `halvor build cli` detects your OS and builds native targets
- **Production releases**: Use GitHub Actions (builds all platforms natively)
- **Cross-platform**: Use `--platforms apple,linux,windows` (not recommended, see below)

**Default Behavior:**
```bash
halvor build cli  # Auto-detects your OS and builds native targets
                  # macOS: aarch64 + x86_64 (darwin)
                  # Linux: x86_64 + aarch64 (gnu/musl)
                  # Windows: x86_64 + aarch64 (msvc)
```

**Why not cross-compile locally?**
Cross-compilation from macOS to Linux/Windows is complicated:
- Requires Docker + `cross` tool
- `cross` can have Docker connection issues
- C dependencies (ring, zstd-sys) cause build failures
- Much slower than native builds

**Recommended**: Use GitHub Actions for multi-platform builds. It's simpler and more reliable.

**Recommended Approach: Use GitHub Actions**

The project has GitHub Actions workflows that build on native runners:
- `.github/workflows/build-linux.yml` - Builds Linux binaries on Ubuntu
- `.github/workflows/build-macos.yml` - Builds macOS binaries on macOS
- `.github/workflows/build-windows.yml` - Builds Windows binaries on Windows

This is the **simplest and most reliable** approach. No cross-compilation needed - each platform builds natively.

**Local Cross-Compilation (Not Recommended)**

Cross-compiling from macOS to Linux/Windows is technically possible but unreliable:

1. **Install dependencies**:
   ```bash
   make install  # Installs cross and all targets
   ```

2. **Ensure Docker is running**:
   ```bash
   docker ps  # Should work without errors
   ```

3. **Attempt build** (may fail):
   ```bash
   halvor build cli --platforms linux,windows
   ```

**Common Issues:**
- `cross` may fail to connect to Docker
- C dependencies (ring, zstd-sys) often fail to compile
- Windows targets have limited `cross` support
- Builds are much slower than native compilation
- **RUSTFLAGS environment variable**: If you have `RUSTFLAGS` set in `~/.zshrc` or `~/.bashrc`, it will break `cross`. The build system automatically clears this variable.

**Cross.toml Configuration:**
The project includes a `Cross.toml` that explicitly specifies Docker images for each target. If cross-compilation fails, the images may need to be pulled first:
```bash
docker pull ghcr.io/cross-rs/x86_64-unknown-linux-gnu:latest
```

**Better Alternative:**
Use GitHub Actions workflows which build natively on each platform. Much more reliable and faster.

## Architecture

### Core Structure

**Commands** (`src/commands/`) are CLI entry points that delegate to **Services** (`src/services/`). This separation enables FFI exports - services can be called from Swift, Kotlin, or WASM without CLI dependencies.

```
User invokes CLI → main.rs → commands/*.rs → services/*.rs → utils/*.rs
                                            ↓
                                      FFI bindings (ffi/*.rs)
                                            ↓
                      Swift/Kotlin/WASM wrappers call services directly
```

### Key Modules

- **`src/commands/`** - CLI command handlers (agent, backup, build, config, dev, install, npm, provision, smb, sync, tailscale, uninstall, update). Each command parses CLI args and calls corresponding service.

- **`src/services/`** - Business logic implementations. These are platform-agnostic and exported via FFI. Organized by feature (backup, build/, dev/, docker/, npm, portainer, provision, smb, sync, tailscale, web).

- **`src/config/`** - Configuration management using TOML files and environment variables. Supports encrypted data via `config_manager.rs` and `.env` file parsing via `env_file.rs`.

- **`src/db/`** - SQLite database abstraction with migrations. Schema is in `migrations/`, generated table code in `generated/`, and core traits in `core/`. Run migrations via `db::migrate::run_migrations()`. Current version: 004.

- **`src/agent/`** - HTTP server (Axum) for agent daemon mode. Provides REST API for remote command execution, service discovery, and sync operations. Start with `halvor agent start`.

- **`src/ffi/`** - Foreign Function Interface layer. `c_ffi.rs` defines C exports, `client.rs` provides FFI client library. Uses custom `#[multi_platform_export]` macro from `halvor-ffi-macro` crate.

- **`src/utils/`** - Cross-cutting utilities: `exec.rs` (command execution), `ssh.rs` (SSH operations), `crypto.rs` (AES-GCM encryption), `networking.rs`, `ffi_bindings.rs` (FFI code generation).

### FFI Integration

The codebase uses a custom FFI system to share Rust code across platforms:

1. **Macro-based exports**: Annotate functions with `#[multi_platform_export]` from `crates/halvor-ffi-macro/`
2. **Swift bindings**: Built via `halvor-swift/build.sh`, generates UniFFI bindings and XCFramework
3. **Android bindings**: JNI layer for Kotlin integration
4. **WASM bindings**: wasm-bindgen for TypeScript/JavaScript

When adding new service functions that should be available on mobile/web, annotate them with the FFI macro and they'll be automatically exported.

### Database Migrations

Database schema changes require new migration files:
1. Create new migration in `src/db/migrations/` with version number (e.g., `005_description.sql`)
2. Update version constant in `src/db/migrate.rs`
3. Run migrations automatically on startup or manually via `db::migrate::run_migrations()`

Tables are defined in `src/db/generated/`. Modify these carefully as they're used throughout the codebase.

## Build System

### Cross-Compilation

The build system supports full cross-compilation across all platforms:

- **Native builds**: Uses `cargo build --target <target>` for same-OS builds
- **Cross-OS compilation**: Automatically uses `cross` tool (Docker-based) for cross-OS builds (e.g., macOS -> Linux/Windows)
- **Automatic detection**: The build system automatically detects when `cross` is needed and uses it

**Prerequisites for cross-compilation:**
- Install `cross`: `cargo install cross --git https://github.com/cross-rs/cross` or `brew install cross` (macOS)
- Docker must be running (required by `cross`)

Supported targets:
- Linux: x86_64-gnu, x86_64-musl, aarch64-gnu, aarch64-musl
- macOS: x86_64-darwin, aarch64-darwin
- Windows: x86_64-windows-msvc, aarch64-windows-msvc

**Example**: Building from macOS for Linux:
```bash
halvor build cli --platforms linux  # Automatically uses 'cross' for Linux targets
halvor build cli --platforms apple,linux,windows  # Builds all platforms
```

### CI/CD Workflows

GitHub Actions workflows in `.github/workflows/`:
- `build-linux.yml` - Multi-arch Linux builds
- `build-macos.yml` - macOS Universal binaries
- `build-windows.yml` - Windows x86_64 and ARM64
- `build-pia-vpn.yml` - Docker container builds

Releases are published to GitHub Releases. Docker images are pushed to GitHub Container Registry (ghcr.io).

## Configuration

### Environment Setup

The project uses **direnv** + **1Password** for secrets management:

1. Copy `.envrc.example` to `.envrc`
2. Configure 1Password vault reference
3. Allow direnv: `direnv allow`
4. Environment variables are loaded from 1Password vault

### Configuration Files

- **TOML files** - Service-specific configuration in `config/` or local `.hal/` directory
- **Environment variables** - Loaded via direnv or `.env` files
- **SQLite database** - Persistent storage for host info, SMB servers, encrypted env data

Access configuration via `config::config_manager::ConfigManager` API.

## Key Dependencies

### Core Libraries
- **clap 4.5** - CLI argument parsing with derive macros
- **tokio 1** - Async runtime with multi-thread support
- **axum 0.7** - Web framework for agent server
- **rusqlite 0.31** - SQLite with bundled binary
- **aes-gcm 0.10** - Encryption for sensitive data
- **reqwest 0.12** - HTTP client with rustls-tls

### Serialization
- **serde 1.0** - Serialization framework
- **serde_json**, **toml 0.9**, **yaml-rust 0.4** - Format parsers

### Platform Utilities
- **nix 0.28** - Unix system calls (signal, process)
- **whoami 1.4** - User/host detection
- **uuid 1.10** - UUID generation with v4

## Docker Compose Files

Pre-configured Docker Compose files are in `compose/`:
- `media.docker-compose.yml` - SABnzbd, qBittorrent, Radarr, Sonarr
- `portainer.docker-compose.yml` - Container management UI
- `nginx-proxy-manager.docker-compose.yml` - Reverse proxy with SSL
- `openvpn-pia.docker-compose.yml` - PIA VPN container

Deploy services via `halvor install <service>` commands.

## Testing

No formal test framework is currently implemented. To add tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        // Test implementation
    }
}
```

Run tests: `cargo test`

## Common Patterns

### Adding a New Command

1. Add variant to `Commands` enum in `src/lib.rs`
2. Create handler in `src/commands/new_command.rs`
3. Implement service logic in `src/services/new_command.rs`
4. Export in `src/commands/mod.rs` and `src/services/mod.rs`
5. Handle command in `src/main.rs` match statement

### Adding FFI-Exported Function

1. Implement function in service module
2. Annotate with `#[multi_platform_export]`
3. Rebuild platform bindings:
   - Swift: `cd halvor-swift && ./build.sh`
   - Android: `halvor build android`
   - Web: `halvor build web`

### Working with Config

```rust
use crate::config::config_manager::ConfigManager;

let config = ConfigManager::load()?;
let value = config.get("key")?;
config.set("key", "value")?;
config.save()?;
```

### Database Operations

```rust
use crate::db::core::Database;
use crate::db::generated::host_info::HostInfo;

let db = Database::new("path/to/db.sqlite")?;
let host = HostInfo::create(&db, "hostname", "192.168.1.1")?;
let hosts = HostInfo::list_all(&db)?;
```

### Running Remote Commands via SSH

```rust
use crate::utils::ssh::run_ssh_command;

let output = run_ssh_command("hostname", "ls -la /path", None)?;
println!("Output: {}", output);
```

## Platform-Specific Notes

### iOS/macOS (Swift)
- Build script: `halvor-swift/build.sh`
- Generates XCFramework from Rust static library
- Swift Package Manager configuration in `halvor-swift/Package.swift`
- Xcode project generated via xcodegen

### Android (Kotlin)
- Gradle project in `halvor-android/`
- Uses NDK for native library compilation
- JNI bindings generated automatically

### Web (TypeScript/Svelte)
- SvelteKit application in `halvor-web/`
- Vite configuration: `halvor-web/vite.config.ts`
- WASM module built with wasm-pack
- Multi-stage Dockerfile for production builds
- Development: `halvor dev web` (Docker) or `halvor dev web --bare-metal`

## Debugging

### Enable Verbose Logging
Most commands support `--verbose` flag for detailed output.

### Agent Server Logs
The agent server runs on port 3030 by default. Check logs for HTTP requests and service operations.

### Database Inspection
```bash
sqlite3 ~/.hal/halvor.db
.tables
.schema
SELECT * FROM host_info;
```

## Installation Scripts

Universal installers detect platform and download pre-built binaries:
- Unix/macOS/Linux: `scripts/install.sh`
- Windows: `scripts/install.ps1`

Binaries are installed to `/usr/local/bin` (Unix) or `~/.local/bin` (Windows).
