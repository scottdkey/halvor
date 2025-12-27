# Halvor Codebase Refactoring Plan

## Current Issues

1. **Monolithic structure** - Everything is in `projects/core`
2. **Mixed concerns** - Build tools, dev tools, CLI runtime, and agent server all in one binary
3. **Large release binary** - Includes development-only code
4. **Unclear boundaries** - Hard to understand what depends on what

## Proposed Architecture

### Workspace Structure

```
halvor/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── halvor-core/              # Core library (shared by all)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config/           # Configuration management
│   │   │   ├── db/               # Database layer
│   │   │   ├── services/         # Business logic (docker, ssh, tailscale, etc.)
│   │   │   └── utils/            # Utilities (crypto, exec, networking)
│   │
│   ├── halvor-cli/               # CLI binary (runtime only)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/         # Runtime commands (status, config, install, etc.)
│   │
│   ├── halvor-agent/             # Agent server binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── server.rs         # HTTP/TCP server
│   │   │   ├── mesh.rs           # Mesh protocol
│   │   │   ├── discovery.rs      # Peer discovery
│   │   │   └── sync.rs           # Data synchronization
│   │
│   ├── halvor-build/             # Build tooling binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── cli.rs            # Build CLI binaries
│   │   │   ├── ios.rs            # Build iOS app
│   │   │   ├── android.rs        # Build Android app
│   │   │   ├── web.rs            # Build web UI
│   │   │   └── ffi.rs            # Build FFI bindings
│   │
│   ├── halvor-dev/               # Development tooling binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── watch.rs          # Hot reload
│   │   │   ├── ios.rs            # iOS simulator
│   │   │   ├── android.rs        # Android emulator
│   │   │   └── web.rs            # Web dev server
│   │
│   ├── halvor-web/               # Web UI (Axum + static files)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs           # Web server
│   │   │   ├── api.rs            # API routes
│   │   │   └── handlers.rs       # Request handlers
│   │
│   └── halvor-ffi/               # FFI bindings
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   ├── swift.rs          # Swift/iOS bindings
│       │   ├── kotlin.rs         # Kotlin/Android bindings
│       │   └── wasm.rs           # WASM bindings
│
└── projects/
    ├── ios/                      # iOS app (Swift)
    ├── android/                  # Android app (Kotlin)
    └── web/                      # Web UI (SvelteKit)
```

## Module Boundaries

### 1. `halvor-core` (Library)

**Purpose**: Shared business logic and utilities

**Exports**:
- Configuration management
- Database operations
- Service implementations (Docker, SSH, Tailscale, K3s, etc.)
- Utilities (crypto, exec, networking)

**Dependencies**: Minimal external dependencies

**No**:
- CLI argument parsing
- Build tools
- Dev tools
- FFI bindings

### 2. `halvor-cli` (Binary)

**Purpose**: User-facing CLI for runtime operations

**Commands**:
- `halvor status` - Show system status
- `halvor config` - Manage configuration
- `halvor install <service>` - Install services
- `halvor uninstall <service>` - Uninstall services
- `halvor update` - Update halvor itself
- `halvor init` - Initialize configuration

**Excludes**:
- `halvor build` (moved to halvor-build)
- `halvor dev` (moved to halvor-dev)

**Dependencies**:
- `halvor-core`
- `clap` (CLI parsing)

**Size**: Small (~5-10MB) - suitable for release

### 3. `halvor-agent` (Binary)

**Purpose**: Agent server for mesh networking

**Features**:
- HTTP/TCP server on port 13500
- Mesh protocol implementation
- Peer discovery
- Database synchronization
- File transfer
- Media streaming

**Can run**:
- Standalone daemon
- In Docker container
- As systemd service
- Embedded in `halvor-cli` (via subprocess)

**Dependencies**:
- `halvor-core`
- `axum` (web framework)
- `tokio` (async runtime)

**Size**: Medium (~10-15MB)

### 4. `halvor-build` (Binary)

**Purpose**: Build all platform targets

**Commands**:
- `halvor-build cli [--platforms apple,linux,windows]`
- `halvor-build ios [--release]`
- `halvor-build android [--release]`
- `halvor-build web [--prod]`
- `halvor-build ffi [--platforms ios,android,wasm]`
- `halvor-build all` - Build everything

**Features**:
- Cross-compilation
- Docker-based builds for Linux/Windows
- FFI generation
- Platform-specific packaging

**Dependencies**:
- `halvor-core`
- Build tools (cargo, docker, xcodebuild, gradle)

**Size**: Large (~50-100MB with toolchains)

**Distribution**: Development only (not released to users)

### 5. `halvor-dev` (Binary)

**Purpose**: Development mode with hot reload

**Commands**:
- `halvor-dev cli` - Watch and rebuild CLI
- `halvor-dev ios` - Run iOS simulator with hot reload
- `halvor-dev android` - Run Android emulator with hot reload
- `halvor-dev web [--bare-metal]` - Run web dev server
- `halvor-dev agent` - Run agent with auto-restart

**Features**:
- File watching (cargo-watch)
- Auto-rebuild on changes
- Simulator/emulator management
- Live reload

**Dependencies**:
- `halvor-core`
- `halvor-build` (reuses build logic)
- `cargo-watch`
- Platform SDKs

**Size**: Large (~50-100MB)

**Distribution**: Development only

### 6. `halvor-web` (Binary)

**Purpose**: Web UI server

**Features**:
- Axum HTTP server
- Serves static SvelteKit build
- API proxy to agent
- WebSocket support for streaming

**Runs**:
- Standalone (port 3000)
- Embedded in agent (shared port)
- In Docker container

**Dependencies**:
- `halvor-core`
- `axum`
- Static file serving

**Size**: Small (~5-10MB) + static assets

### 7. `halvor-ffi` (Library)

**Purpose**: FFI bindings for mobile/web

**Exports**:
- Swift bindings (iOS/macOS)
- Kotlin bindings (Android)
- WASM bindings (Web)

**Uses**:
- UniFFI for Swift/Kotlin
- wasm-bindgen for WASM

**Dependencies**:
- `halvor-core`
- `uniffi`
- `wasm-bindgen`

## Migration Strategy

### Phase 1: Create Workspace (Week 1)

1. Create `Cargo.toml` workspace root
2. Move current code to `crates/halvor-core`
3. Extract CLI to `crates/halvor-cli`
4. Extract build commands to `crates/halvor-build`
5. Update all imports

### Phase 2: Extract Agent (Week 2)

1. Create `crates/halvor-agent`
2. Move agent code from core
3. Update systemd service to use `halvor-agent` binary
4. Update `halvor-cli` to spawn `halvor-agent` subprocess

### Phase 3: Extract Dev Tools (Week 3)

1. Create `crates/halvor-dev`
2. Move dev commands from core
3. Reuse build logic from `halvor-build`
4. Update Makefile

### Phase 4: Extract Web (Week 4)

1. Create `crates/halvor-web`
2. Move web server code
3. Update Docker configuration
4. Test standalone and embedded modes

### Phase 5: Clean Up (Week 5)

1. Remove unused code from core
2. Minimize dependencies
3. Optimize binary sizes
4. Update documentation

## Command Mapping

### Current → New

```bash
# CLI (halvor-cli)
halvor status              → halvor status
halvor config              → halvor config
halvor install nginx       → halvor install nginx
halvor uninstall nginx     → halvor uninstall nginx
halvor update              → halvor update
halvor init                → halvor init

# Agent (halvor-agent or halvor-cli with agent subcommand)
halvor agent start         → halvor agent start (spawns halvor-agent)
halvor agent stop          → halvor agent stop
halvor agent status        → halvor agent status
halvor agent token         → halvor agent token
halvor agent join          → halvor agent join
halvor agent peers         → halvor agent peers
halvor agent sync          → halvor agent sync

# Build (halvor-build) - development only
halvor build cli           → halvor-build cli
halvor build ios           → halvor-build ios
halvor build android       → halvor-build android
halvor build web           → halvor-build web

# Dev (halvor-dev) - development only
halvor dev cli             → halvor-dev cli
halvor dev ios             → halvor-dev ios
halvor dev android         → halvor-dev android
halvor dev web             → halvor-dev web
```

## Binary Sizes (Estimated)

| Binary | Size | Distribution |
|--------|------|--------------|
| `halvor` (CLI) | ~8MB | Public release |
| `halvor-agent` | ~12MB | Public release |
| `halvor-web` | ~8MB | Public release |
| `halvor-build` | ~80MB | Development only |
| `halvor-dev` | ~80MB | Development only |
| `halvor-ffi` | ~15MB | Build artifact |

## Benefits

1. **Smaller releases** - Users only download runtime binaries
2. **Clear separation** - Each binary has a single purpose
3. **Better testing** - Each crate can be tested independently
4. **Faster builds** - Only rebuild what changed
5. **Easier onboarding** - Clearer structure for contributors
6. **Docker optimization** - Smaller images with only needed binaries

## Docker Changes

### Current (monolithic)
```dockerfile
FROM rust:latest
COPY . .
RUN cargo build --release
# Result: ~500MB image with dev tools
```

### New (modular)
```dockerfile
# Build stage
FROM rust:latest AS builder
COPY . .
RUN cargo build --release --bin halvor-agent --bin halvor-web

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/halvor-agent /usr/local/bin/
COPY --from=builder /app/target/release/halvor-web /usr/local/bin/
# Result: ~100MB image with only runtime binaries
```

## Web UI Access

The web UI can access the agent in multiple ways:

1. **Same process**: Agent starts web server on different port
   ```bash
   halvor agent start --web-port 3000
   # Agent on :13500, Web UI on :3000
   ```

2. **Separate processes**: Web UI proxies to agent
   ```bash
   halvor-agent start                    # :13500
   halvor-web start --agent-url http://localhost:13500  # :3000
   ```

3. **Docker**: Use container networking
   ```yaml
   services:
     agent:
       image: halvor-agent
       ports: ["13500:13500"]
     web:
       image: halvor-web
       environment:
         AGENT_URL: "http://agent:13500"
       ports: ["3000:3000"]
   ```

## Next Steps

1. Review this plan
2. Get approval for architecture
3. Start Phase 1 (create workspace)
4. Implement incrementally
5. Update CI/CD pipelines
6. Update documentation

## Questions to Resolve

1. Should `halvor-agent` be embedded in `halvor-cli` or separate binary?
2. Keep `halvor` as umbrella command or split into separate binaries?
3. How to handle agent updates when running as systemd service?
4. FFI: Build as part of main workspace or separate?
