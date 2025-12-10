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

- **[Configuration Guide](docs/configuration.md)** - Setting up your environment file and managing
  configuration
- **[Setup Guide](docs/setup.md)** - SSH host configuration and key setup
- **[Usage Guide](docs/usage.md)** - Common commands and operations
- **[Development Guide](docs/development.md)** - Building, testing, and contributing
- **[Workflows](docs/workflows.md)** - GitHub Actions CI/CD documentation
- **[IPv6 Setup](docs/ipv6-setup.md)** - Enabling IPv6 support in VPN container

## Quick Start

After installation:

1. **Configure HAL**: `hal config init`
2. **Setup SSH hosts**: `./scripts/setup-ssh-hosts.sh`
3. **Setup SSH keys**: `./scripts/setup-ssh-keys.sh <hostname>`
4. **Connect**: `ssh <hostname>`

See the [Configuration Guide](docs/configuration.md) for detailed setup instructions.
