# Development Guide

## Building

```bash
# Build release binary
cargo build --release

# Build for specific platform
halvor build cli --platforms linux
```

## Running tests

```bash
cargo test
```

## Development mode with auto-rebuild

```bash
# CLI development with watch mode
halvor dev cli
# or
make dev
```

This uses `cargo-watch` to automatically rebuild and reinstall when files change.

For platform-specific development, see [Multi-Platform Guide](multi-platform.md).

## Project Structure

```
.
├── Cargo.toml          # Rust project configuration
├── Makefile            # Development commands
├── scripts/            # Installation and setup scripts
│   ├── install.sh      # Unix/macOS/Linux installation script
│   ├── install.ps1     # Windows PowerShell installation script
│   └── generate-docs.sh # Documentation generation
├── src/
│   ├── main.rs         # Main CLI binary entry point
│   ├── lib.rs          # Library crate (exposes modules)
│   ├── commands/       # CLI command handlers
│   │   ├── agent.rs    # Agent daemon commands
│   │   ├── backup.rs   # Backup/restore commands
│   │   ├── build.rs    # Build commands
│   │   ├── config.rs   # Configuration commands
│   │   ├── dev.rs      # Development commands
│   │   ├── install.rs  # Installation commands
│   │   ├── init.rs     # Cluster initialization
│   │   ├── join.rs     # Cluster join
│   │   ├── status.rs   # Status commands
│   │   ├── sync.rs     # Sync commands
│   │   ├── tailscale.rs # Tailscale commands
│   │   └── ...
│   ├── services/       # Business logic (platform-agnostic)
│   │   ├── backup/     # Backup service
│   │   ├── build/      # Build service
│   │   ├── dev/        # Development service
│   │   ├── docker/     # Docker service
│   │   ├── k3s/        # K3s cluster service
│   │   ├── smb/        # SMB service
│   │   ├── sync/       # Sync service
│   │   ├── tailscale/  # Tailscale service
│   │   └── web/        # Web service
│   ├── config/         # Configuration management
│   │   ├── config_manager.rs # Config manager
│   │   ├── env_file.rs # .env file parsing
│   │   └── service.rs  # Config service
│   ├── db/             # SQLite database
│   │   ├── core/       # Database core traits
│   │   ├── generated/  # Generated table code
│   │   ├── migrations/ # Database migrations
│   │   └── migrate.rs  # Migration runner
│   ├── agent/          # Agent daemon (HTTP server)
│   │   ├── api.rs      # API handlers
│   │   ├── client.rs   # Agent client
│   │   ├── discovery.rs # Service discovery
│   │   ├── server.rs   # HTTP server
│   │   └── sync.rs     # Sync operations
│   ├── ffi/            # Foreign Function Interface
│   │   ├── c_ffi.rs    # C FFI exports
│   │   └── mod.rs      # FFI module
│   └── utils/          # Cross-cutting utilities
│       ├── exec.rs     # Command execution
│       ├── ssh.rs      # SSH operations
│       ├── crypto.rs   # Encryption
│       └── ...
├── halvor-swift/       # Swift/iOS/macOS bindings
│   ├── Package.swift   # Swift Package Manager
│   ├── build.sh        # Build script
│   └── Sources/        # Swift source code
├── halvor-android/     # Android/Kotlin bindings
│   ├── build.gradle.kts # Gradle configuration
│   └── src/            # Kotlin source code
├── halvor-web/         # Web/WASM application
│   ├── package.json    # Node.js dependencies
│   ├── vite.config.ts  # Vite configuration
│   └── src/            # SvelteKit source code
├── openvpn-container/  # VPN Docker container
├── compose/            # Docker Compose files
├── charts/             # Helm charts
├── docs/                # Documentation
├── .env                # Environment configuration (gitignored)
└── README.md           # Main README
```

## Requirements

- Rust (latest stable) - automatically installed by install scripts if not present
- `cargo-watch` (for development mode, installed automatically)
- SSH client
- Tailscale (optional, for Tailscale connections) - can be installed via `halvor tailscale install`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request
