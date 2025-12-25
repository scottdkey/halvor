# HAL - Homelab Automation Layer

## Status Badges

### CI/CD & Build Status

[![CI Workflow](https://github.com/scottdkey/homelab/actions/workflows/ci.yml/badge.svg)](https://github.com/scottdkey/homelab/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=latest%20release&logo=github&sort=semver)](https://github.com/scottdkey/homelab/releases/latest)
[![Release Date](https://img.shields.io/github/release-date/scottdkey/homelab?label=released&logo=github)](https://github.com/scottdkey/homelab/releases)

### Docker Image

[![Docker Build Status](https://img.shields.io/github/actions/workflow/status/scottdkey/homelab/build-pia-vpn.yml?label=PIA%20VPN%20build&logo=docker)](https://github.com/scottdkey/homelab/actions/workflows/build-pia-vpn.yml)
[![Docker Image](https://img.shields.io/badge/docker-ghcr.io%2Fscottdkey%2Fpia--vpn-blue?logo=docker)](https://github.com/users/scottdkey/packages/container/package/pia-vpn)
[![Docker Image Version](https://img.shields.io/github/v/release/scottdkey/homelab?label=docker%20version&logo=docker&sort=semver)](https://github.com/scottdkey/homelab/pkgs/container/pia-vpn)

### Platform Releases

#### Linux

[![Linux Build Status](https://img.shields.io/github/actions/workflow/status/scottdkey/homelab/build-linux.yml?label=Linux%20build&logo=linux)](https://github.com/scottdkey/homelab/actions/workflows/build-linux.yml)
[![Linux x86_64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=Linux%20x86_64&logo=linux&sort=semver)](https://github.com/scottdkey/homelab/releases)
[![Linux ARM64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=Linux%20ARM64&logo=linux&sort=semver)](https://github.com/scottdkey/homelab/releases)
[![Linux RISC-V Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=Linux%20RISC-V&logo=linux&sort=semver)](https://github.com/scottdkey/homelab/releases)

#### macOS

[![macOS Build Status](https://img.shields.io/github/actions/workflow/status/scottdkey/homelab/build-macos.yml?label=macOS%20build&logo=apple)](https://github.com/scottdkey/homelab/actions/workflows/build-macos.yml)
[![macOS x86_64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=macOS%20x86_64&logo=apple&sort=semver)](https://github.com/scottdkey/homelab/releases)
[![macOS ARM64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=macOS%20ARM64&logo=apple&sort=semver)](https://github.com/scottdkey/homelab/releases)

#### Windows

[![Windows Build Status](https://img.shields.io/github/actions/workflow/status/scottdkey/homelab/build-windows.yml?label=Windows%20build&logo=windows)](https://github.com/scottdkey/homelab/actions/workflows/build-windows.yml)
[![Windows x86_64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=Windows%20x86_64&logo=windows&sort=semver)](https://github.com/scottdkey/homelab/releases)
[![Windows ARM64 Release](https://img.shields.io/github/v/release/scottdkey/homelab?label=Windows%20ARM64&logo=windows&sort=semver)](https://github.com/scottdkey/homelab/releases)

A Rust-based CLI tool for managing your homelab infrastructure, with scripts for SSH setup and
automation.

**HAL** stands for **Homelab Automation Layer** - your intelligent assistant for homelab operations.

## Installation

### Automatic Installation (Recommended)

Download and run the install script from GitHub:

**On Unix/macOS/Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/install.sh | bash
```

Or download and run manually:

```bash
curl -O https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/install.sh
chmod +x install.sh
./install.sh
```

**On Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/install.ps1 | iex
```

The install scripts will:

- Detect your platform (OS and architecture)
- Download the correct pre-built binary from GitHub releases
- Install to `/usr/local/bin` (Linux/macOS) or `~/.local/bin` (Windows)
- Set up PATH if needed

### Manual Installation

If you already have Rust installed:

```bash
cargo install --path .
```

Or using make:

```bash
make install
```

### Development Mode

For development, use the watch mode that automatically rebuilds and installs on changes:

```bash
make dev
```

This will:

- Watch for changes in the source code
- Automatically rebuild the project
- Install the binary globally

## Documentation

### Auto-Generated Documentation

These documents are automatically generated and kept up-to-date:

- **[CLI Commands Reference](docs/generated/cli-commands.md)** - Complete reference of all `halvor` CLI commands and options
- **[Docker Containers](docs/generated/docker-containers.md)** - Available Docker containers and how to use them
- **[Helm Charts](docs/generated/helm-charts.md)** - Available Helm charts and installation instructions

To regenerate these docs locally:
```bash
make docs
# or
./scripts/generate-docs.sh
```

### Manual Documentation

**Setup & Configuration:**
- **[Configuration Guide](docs/configuration.md)** - Setting up your environment file and managing configuration
- **[Setup Guide](docs/setup.md)** - Initial setup and SSH configuration
- **[Cluster Setup Guide](docs/cluster-setup.md)** - Setting up K3s Kubernetes cluster

**Development:**
- **[Development Guide](docs/development.md)** - Building, testing, and contributing
- **[Multi-Platform Guide](docs/multi-platform.md)** - Building for iOS, Android, and Web

**Architecture & Advanced:**
- **[Agent Architecture](docs/agent-architecture.md)** - Understanding the agent mesh network
- **[VPN Setup](docs/vpn-setup.md)** - PIA VPN container setup and configuration
- **[VPN Troubleshooting](docs/vpn-troubleshooting.md)** - Common VPN issues and solutions
- **[VPN Routing](docs/vpn-routing.md)** - VPN routing configuration
- **[IPv6 Setup](docs/ipv6-setup.md)** - Enabling IPv6 support in VPN container
- **[Workflows](docs/workflows.md)** - GitHub Actions CI/CD documentation

## Quick Start

After installation:

1. **Configure HAL**: `halvor config init`
2. **Setup SSH hosts**: `./scripts/setup-ssh-hosts.sh`
3. **Setup SSH keys**: `./scripts/setup-ssh-keys.sh <hostname>`
4. **Connect**: `ssh <hostname>`

See the [Configuration Guide](docs/configuration.md) for detailed setup instructions.

## Key Features

- **K3s Cluster Management**: Initialize, join, and manage Kubernetes clusters
- **Docker Container Building**: Build and push containers to GitHub Container Registry
- **Helm Chart Deployment**: Deploy and manage Helm charts on your cluster
- **SMB Share Management**: Mount and manage SMB shares across your infrastructure
- **VPN Integration**: Deploy and manage PIA VPN containers with Rust-based entrypoint
- **Agent Daemon**: Remote command execution and service discovery
- **Multi-Platform Support**: Build for macOS, iOS, Android, Web (WASM), and CLI

For detailed command reference, see the [CLI Commands Reference](docs/generated/cli-commands.md).
