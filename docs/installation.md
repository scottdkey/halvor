# Installation Guide

This guide covers installing halvor on your system and performing initial setup.

## Installing halvor

### Automatic Installation (Recommended)

**On Unix/macOS/Linux:**

```bash
curl -fsSL https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.sh | bash
```

Or download and run manually:

```bash
curl -O https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.sh
chmod +x install.sh
./install.sh
```

**On Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.ps1 | iex
```

The install scripts will:
- Detect your platform (OS and architecture)
- Download the correct pre-built binary from GitHub releases
- Install to `/usr/local/bin` (Linux/macOS) or `~/.local/bin` (Windows)
- Set up PATH if needed

### Manual Installation from Source

If you have Rust installed:

```bash
# Clone the repository
git clone https://github.com/scottdkey/halvor.git
cd halvor

# Build and install
cargo build --release
cargo install --path . --bin halvor --force
```

Or using make:

```bash
make install-cli
```

### Development Installation

For development with auto-rebuild:

```bash
make dev
# or
halvor dev cli
```

This uses `cargo-watch` to automatically rebuild and reinstall when files change.

## Initial Configuration

### 1. Configure halvor

Initialize halvor configuration:

```bash
halvor config init
```

This sets up the path to your `.env` file and initializes the configuration database.

### 2. Environment Configuration

Halvor uses environment variables loaded from a `.env` file. The recommended approach is using **direnv** + **1Password** for secure secret management.

**Using direnv + 1Password (Recommended):**

1. **Setup direnv** (if not already installed):
   ```bash
   # macOS
   brew install direnv
   
   # Linux
   sudo apt install direnv
   ```

2. **Configure `.envrc`** to load secrets from 1Password:
   ```bash
   # Copy example
   cp .envrc.example .envrc
   
   # Edit .envrc with your 1Password vault reference
   # Then allow direnv
   direnv allow
   ```

3. **Environment variables are automatically loaded** when you enter the directory.

**Manual .env File:**

Alternatively, create a `.env` file manually:

```bash
# Create example file
halvor config env

# Or manually create .env in your project directory
```

### 3. Host Configuration

Add host configurations to your `.env` file:

```bash
# Tailscale base domain
TAILSCALE_DOMAIN="ts.net"

# Host configurations
HOST_FRIGG_IP="10.10.10.10"
HOST_FRIGG_HOSTNAME="frigg.ts.net"
HOST_FRIGG_BACKUP_PATH="/mnt/backups/frigg"

HOST_BAULDER_IP="10.10.10.11"
HOST_BAULDER_HOSTNAME="baulder.ts.net"
HOST_BAULDER_BACKUP_PATH="/mnt/backups/baulder"
```

For detailed configuration options, see the [Configuration Guide](configuration.md).

## Basic Commands

After installation, you can start using halvor:

### List Available Apps

```bash
halvor install --list
```

### Install Platform Tools

```bash
# Install Docker
halvor install docker -H frigg

# Install Tailscale
halvor install tailscale -H frigg

# Install SMB mounts
halvor install smb -H frigg
```

### Install Helm Charts

```bash
# Install Portainer (automatically detected as Helm chart)
halvor install portainer -H frigg

# Install Gitea
halvor install gitea -H frigg
```

The CLI automatically detects Helm charts - no `--helm` flag needed.

### Initialize K3s Cluster

```bash
# Initialize primary control plane node
halvor init -H frigg

# Join additional nodes
halvor join -H baulder --server=frigg --control-plane
```

### Check Status

```bash
# Check K3s cluster status
halvor status k3s -H frigg

# Check Helm releases
halvor status helm -H frigg
```

## Next Steps

- **Detailed Commands**: See [Usage Guide](usage.md) for comprehensive command reference
- **Cluster Setup**: See [Cluster Setup Guide](cluster-setup.md) for complete K3s cluster setup
- **Configuration**: See [Configuration Guide](configuration.md) for detailed configuration options
- **Development**: See [Development Guide](development.md) if you want to contribute

## Troubleshooting

### Installation Issues

If the installation script fails:

1. **Check your platform**: Ensure you're on a supported platform (Linux, macOS, Windows)
2. **Check network**: Ensure you can access GitHub releases
3. **Manual install**: Try building from source if automatic installation fails

### Configuration Issues

If configuration fails:

1. **Check .env file**: Ensure your `.env` file exists and is readable
2. **Check permissions**: Ensure halvor can write to the configuration directory
3. **Check logs**: Run with verbose output: `halvor config --verbose`

### Command Not Found

If `halvor` command is not found:

1. **Check PATH**: Ensure the installation directory is in your PATH
2. **Reload shell**: Restart your terminal or run `source ~/.bashrc` (or equivalent)
3. **Verify installation**: Check that the binary exists at the installation path

