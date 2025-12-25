# Usage Guide

Complete reference for all halvor commands and options. For quick examples, see the [Installation Guide](installation.md).

## Installation Commands

### `halvor install`

Install platform tools or Helm charts on a host.

**Usage:**
```bash
halvor install <app> [-H <hostname>]
halvor install --list
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname (default: localhost for platform tools, frigg for Helm charts)
- `--list` - List all available apps

**Examples:**
```bash
# List all available apps
halvor install --list

# Install platform tools
halvor install docker -H frigg
halvor install tailscale -H frigg
halvor install smb -H frigg
halvor install k3s -H frigg

# Install Helm charts (automatically detected)
halvor install portainer -H frigg
halvor install gitea -H frigg
halvor install nginx-proxy-manager -H frigg
halvor install traefik-public -H frigg
halvor install traefik-private -H frigg
```

**Notes:**
- Helm charts are automatically detected based on app definition - no `--helm` flag needed
- Platform tools default to `localhost` if no hostname is specified
- Helm charts default to `frigg` (primary cluster node) if no hostname is specified
- The CLI validates cluster availability before installing Helm charts

## Backup and Restore Commands

### `halvor backup`

Backup services, configuration, or database.

**Usage:**
```bash
halvor backup [-H <hostname>] [<service>] [--env] [--list] [--db] [--path <path>]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname
- `<service>` - Service to backup (e.g., portainer, sonarr). If not provided, interactive selection
- `--env` - Backup to env location instead of backup path
- `--list` - List available backups instead of creating one
- `--db` - Backup the database (unencrypted SQLite backup)
- `--path <path>` - Path to save database backup (only used with --db)

**Examples:**
```bash
# Backup all services interactively
halvor backup -H frigg

# Backup a specific service
halvor backup portainer -H frigg

# List available backups
halvor backup -H frigg --list

# Backup database
halvor backup --db --path ./backup.db
```

### `halvor restore`

Restore services, configuration, or database from backup.

**Usage:**
```bash
halvor restore [-H <hostname>] [<service>] [--env] [--backup <timestamp>]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname
- `<service>` - Service to restore (e.g., portainer, sonarr). If not provided, interactive selection
- `--env` - Restore from env location instead of backup path
- `--backup <timestamp>` - Specific backup timestamp to restore (required when service is specified)

**Examples:**
```bash
# Restore a service interactively
halvor restore -H frigg

# Restore a specific service from a specific backup
halvor restore portainer -H frigg --backup 2024-01-15T10:30:00
```

## Cluster Management Commands

### `halvor init`

Initialize K3s cluster (primary control plane node).

**Usage:**
```bash
halvor init [-H <hostname>] [--token <token>] [-y]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname (default: localhost)
- `--token <token>` - Cluster join token (generated if not provided)
- `-y, --yes` - Skip confirmation prompts

**Examples:**
```bash
# Initialize cluster on frigg
halvor init -H frigg

# Initialize with custom token
halvor init -H frigg --token my-custom-token

# Initialize without prompts
halvor init -H frigg -y
```

### `halvor join`

Join a node to the K3s cluster.

**Usage:**
```bash
halvor join [-H <hostname>] [<join_hostname>] [--server <server>] [--token <token>] [--control-plane]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname to join
- `<join_hostname>` - Hostname of the node to join (optional, can use -H instead)
- `--server <server>` - First control plane node address (e.g., frigg or 192.168.1.10). If not provided, will try to auto-detect from config
- `--token <token>` - Cluster join token (if not provided, will be loaded from K3S_TOKEN env var or fetched from server)
- `--control-plane` - Join as control plane node (default: false)

**Examples:**
```bash
# Join baulder as control plane node
halvor join -H baulder --server=frigg --control-plane

# Join oak as worker node
halvor join -H oak --server=frigg
```

### `halvor status`

Show status of services or cluster.

**Usage:**
```bash
halvor status <subcommand> [-H <hostname>]
```

**Subcommands:**
- `k3s` - Show K3s cluster status
- `helm` - Show Helm releases

**Examples:**
```bash
# Check K3s cluster status
halvor status k3s -H frigg

# List Helm releases
halvor status helm -H frigg
```

### `halvor configure`

Configure Tailscale integration for K3s cluster.

**Usage:**
```bash
halvor configure [-H <hostname>] [<target_hostname>]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Source hostname
- `<target_hostname>` - Target hostname (default: localhost)

**Examples:**
```bash
# Configure Tailscale on localhost
halvor configure

# Configure Tailscale on remote host
halvor configure -H frigg
```

## Configuration Commands

### `halvor config`

Manage halvor configuration.

**Usage:**
```bash
halvor config [--verbose] [--db] [<subcommand>]
```

**Options:**
- `--verbose` - Show verbose output (including passwords)
- `--db` - Show database configuration instead of .env

**Subcommands:**
- `init` - Initialize or update halvor configuration (interactive)
- `list` - List current configuration
- `set-env <path>` - Set the environment file path
- `stable` - Set release channel to stable
- `experimental` - Set release channel to experimental
- `env` - Create example .env file
- `commit` - Commit host configuration to database (from .env to DB)
- `backup` - Write host configuration back to .env file (from DB to .env, backs up current .env first)
- `diff` - Show differences between .env and database configurations

**Examples:**
```bash
# Initialize configuration
halvor config init

# List current configuration
halvor config list

# Set environment file path
halvor config set-env /path/to/.env

# Show database configuration
halvor config --db list

# Commit .env to database
halvor config commit

# Show differences
halvor config diff
```

## Build Commands

### `halvor build`

Build applications for different platforms.

**Usage:**
```bash
halvor build <subcommand> [options]
```

**Subcommands:**

#### `halvor build cli`

Build CLI binary for different platforms.

**Options:**
- `--platforms <platforms>` - Platforms to build for (comma-separated: apple,windows,linux). If not specified, builds all.
- `--targets <targets>` - Specific targets to build for (comma-separated Rust target triples)
- `--push` - Push built binaries to GitHub releases
- `--experimental` - Build for current platform and push to experimental release

**Examples:**
```bash
# Build for current platform
halvor build cli

# Build for specific platforms
halvor build cli --platforms apple,linux

# Build and push to releases
halvor build cli --push
```

#### `halvor build ios`

Build iOS app (always signed).

**Options:**
- `--push` - Push to App Store Connect after building

**Examples:**
```bash
# Build iOS app
halvor build ios

# Build and push to App Store
halvor build ios --push
```

#### `halvor build mac`

Build macOS app (always signed).

**Examples:**
```bash
halvor build mac
```

#### `halvor build android`

Build Android app (always signed).

**Examples:**
```bash
halvor build android
```

#### `halvor build web`

Build Web app (Rust server + Svelte frontend).

**Options:**
- `--release` - Build for production release
- `--bare-metal` - Build for bare metal (local Rust binary, no Docker)
- `--run` - Run the container after building
- `--docker` - Build Docker container
- `--push` - Push Docker image to GitHub Container Registry

**Examples:**
```bash
# Build web app
halvor build web

# Build production Docker container
halvor build web --docker --release

# Build and push to registry
halvor build web --push
```

#### `halvor build pia-vpn`

Build PIA VPN Docker container.

**Options:**
- `--no-cache` - Build without using cache
- `--push` - Push to GitHub Container Registry (experimental tag)
- `--release` - Push as release (latest tag instead of experimental)

**Examples:**
```bash
# Build VPN container
halvor build pia-vpn

# Build and push to registry
halvor build pia-vpn --push

# Build and push as release
halvor build pia-vpn --push --release
```

## Development Commands

### `halvor dev`

Development mode for different platforms.

**Usage:**
```bash
halvor dev <subcommand> [options]
```

**Subcommands:**

#### `halvor dev cli`

CLI development mode with watch.

**Examples:**
```bash
halvor dev cli
```

#### `halvor dev mac`

macOS development mode with hot reload.

**Examples:**
```bash
halvor dev mac
```

#### `halvor dev ios`

iOS simulator development mode.

**Examples:**
```bash
halvor dev ios
```

#### `halvor dev web`

Web development mode.

**Options:**
- `--bare-metal` - Run in bare metal mode (Rust server + Svelte dev, no Docker)
- `--prod` - Run in production mode (Docker container)
- `--release` - Run production version locally (uses production Docker container)
- `--port <port>` - Port for the web server (default: 3000)
- `--static-dir <dir>` - Directory containing built Svelte app (for production mode)

**Examples:**
```bash
# Docker-based development (recommended)
halvor dev web

# Bare-metal development
halvor dev web --bare-metal

# Production mode
halvor dev web --prod
```

## Other Commands

### `halvor list`

List services or hosts.

**Usage:**
```bash
halvor list [-H <hostname>] [--verbose]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - List services on specific host (default: list all hosts)
- `--verbose` - Show verbose information

**Examples:**
```bash
# List all hosts
halvor list

# List services on a host
halvor list -H frigg

# Verbose output
halvor list --verbose
```

### `halvor sync`

Sync encrypted data between halvor installations.

**Usage:**
```bash
halvor sync [-H <hostname>] [--pull]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname (required)
- `--pull` - Pull data from remote instead of pushing

**Examples:**
```bash
# Push data to remote host
halvor sync -H frigg

# Pull data from remote host
halvor sync -H frigg --pull
```

### `halvor update`

Update halvor or installed apps.

**Usage:**
```bash
halvor update [-H <hostname>] [<app>] [--experimental] [--force]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname
- `<app>` - App to update (e.g., docker, tailscale, portainer). If not provided, updates everything on the system.
- `--experimental` - Use experimental channel for halvor updates (version less, continuously updated)
- `--force` - Force download and install the latest version (skips version check)

**Examples:**
```bash
# Update halvor itself
halvor update

# Update a specific app
halvor update docker -H frigg

# Update using experimental channel
halvor update --experimental
```

### `halvor uninstall`

Uninstall a service from a host or halvor itself.

**Usage:**
```bash
halvor uninstall [-H <hostname>] [<service>]
```

**Options:**
- `-H, --hostname <HOSTNAME>` - Target hostname
- `<service>` - Service to uninstall (e.g., portainer, smb, nginx-proxy-manager). If not provided, guided uninstall of halvor.

**Examples:**
```bash
# Uninstall a service
halvor uninstall smb -H frigg

# Guided uninstall of halvor
halvor uninstall
```

## Global Options

All commands support the following global options:

- `-H, --hostname <HOSTNAME>` - Target hostname for the operation (default: localhost)
- `-h, --help` - Show help message
- `-V, --version` - Show version information

## Getting Help

For more information on any command:

```bash
halvor <command> --help
halvor <command> <subcommand> --help
```

For auto-generated documentation with all available apps and charts, see:
- [CLI Commands Reference](generated/cli-commands.md)
- [Docker Containers](generated/docker-containers.md)
- [Helm Charts](generated/helm-charts.md)
