# HAL - Homelab Automation Layer

A Rust-based CLI tool for managing your homelab infrastructure, with support for SSH connections via local IP or Tailscale.

**HAL** stands for **Homelab Automation Layer** - your intelligent assistant for homelab operations.

## Features

- **SSH Connection**: Connect to hosts via local IP first, with automatic fallback to Tailscale
- **Environment-based Configuration**: Host configurations stored in `.env` file
- **Development Mode**: Auto-build and install on file changes

## Installation

### Automatic Installation (Recommended)

The install scripts will automatically detect if Rust is installed and install it if needed, then build and install hal.

**On Unix/macOS/Linux:**
```bash
./install.sh
```

**On Windows (PowerShell):**
```powershell
.\install.ps1
```

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

## Configuration

1. Copy `.env.example` to `.env`:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your host configurations:
   ```bash
   # Tailscale configuration
   TAILNET_BASE="ts.net"

   # Host configurations
   HOST_bellerophon_IP="10.10.10.14"
   HOST_bellerophon_TAILSCALE="bellerophon"
   ```

## Usage

### SSH to a host

```bash
hal ssh bellerophon
```

With additional SSH arguments:

```bash
hal ssh bellerophon -L 8080:localhost:8080
```

**Fix SSH host key issues:**

If you encounter "host key verification failed" or "host ID mismatch" errors, use the `--fix-keys` flag to remove offending host keys from your `known_hosts` file:

```bash
hal ssh bellerophon --fix-keys
```

This will:
- Prompt you to confirm removal for each configured host address (IP, Tailscale hostname, etc.)
- Remove the offending host keys from `~/.ssh/known_hosts`
- Then attempt to connect via SSH

You can also use the short form:

```bash
hal ssh bellerophon -f
```

**Copy SSH keys for passwordless authentication:**

To copy your SSH public key to a remote host for passwordless authentication:

```bash
hal ssh bellerophon --keys
```

This will:
- Prompt for username (or use default)
- Use `ssh-copy-id` to copy your public key to the remote host
- Prompt for password once during key copy
- After this, future connections will use key-based authentication

**Automatic authentication method selection:**

The SSH command now automatically:
- First tries key-based authentication (no password prompt)
- Falls back to password authentication if keys aren't set up
- Prompts for username/password only when needed

### Install Tailscale

Install Tailscale on your system (supports macOS, Linux, and Windows):

```bash
hal tailscale install
```

This will:
- Detect your operating system
- Use the appropriate package manager (Homebrew on macOS, apt/yum/dnf on Linux)
- Provide instructions for starting Tailscale

### Provision a Remote Host

Provision a remote host with Docker, Tailscale, and Portainer:

```bash
hal provision bellerophon
```

This will:
- Connect to the host via SSH (prompts for username and password)
- Install Docker if not already installed
- Install Tailscale if not already installed
- Install Portainer Agent (or Portainer CE with `--portainer-host` flag)
- Handle all sudo prompts interactively

**Install Portainer CE instead of Agent:**

```bash
hal provision bellerophon --portainer-host
```

This installs the full Portainer CE with web UI instead of just the agent.

### Setup SMB Mounts

Setup and mount SMB shares on a remote host:

```bash
hal smb bellerophon
```

This will:
- Install SMB client utilities (`cifs-utils`)
- Create mount points at `/mnt/smb/{servername}/{sharename}`
- Mount SMB shares using credentials from `.env`
- Add entries to `/etc/fstab` for persistent mounts

**Uninstall SMB mounts:**

```bash
hal smb bellerophon --uninstall
```

### Backup and Restore Docker Volumes

**Create a backup:**

```bash
hal backup bellerophon create
```

This creates a timestamped backup of all Docker volumes and bind mounts in `/mnt/smb/maple/backups/{hostname}/{timestamp}/`.

**List available backups:**

```bash
hal backup bellerophon list
```

**Restore from a backup:**

```bash
hal backup bellerophon restore
```

If no backup name is specified, it will list available backups and prompt you to select one.

Or restore a specific backup:

```bash
hal backup bellerophon restore --backup 20240101_120000
```

### Automatically Setup Nginx Proxy Manager Hosts

Automatically create proxy hosts in Nginx Proxy Manager from a Docker Compose file:

```bash
hal npm bellerophon media.docker-compose.yml
```

This will:
- Parse the compose file to find services with exposed ports
- Connect to Nginx Proxy Manager API (requires `NPM_USERNAME` and `NPM_PASSWORD` in `.env`)
- Create proxy hosts for each service (e.g., `sonarr.local`, `radarr.local`)
- Forward traffic to the host where services are running

**Required environment variables:**

Add to your `.env` file:

```bash
NPM_URL="https://bellerophon:81"  # Optional, defaults to https://{hostname}:81
NPM_USERNAME="admin@example.com"
NPM_PASSWORD="your-password"
```

The command will:
- Skip services that already have proxy hosts configured
- Use the host's IP or Tailscale address for forwarding
- Create domains in the format `{servicename}.local`

## Development

### Building

```bash
cargo build --release
```

### Running tests

```bash
cargo test
```

### Development mode with auto-rebuild

```bash
make dev
```

This uses `cargo-watch` to automatically rebuild and reinstall when files change.

## Project Structure

```
.
├── Cargo.toml          # Rust project configuration
├── Makefile            # Development commands
├── install.sh          # Unix/macOS/Linux installation script
├── install.ps1         # Windows PowerShell installation script
├── src/
│   ├── main.rs        # Main CLI application
│   └── *.sh           # Original bash scripts (archived)
├── .env               # Environment configuration (gitignored)
├── .env.example       # Example environment configuration
└── README.md          # This file
```

## Requirements

- Rust (latest stable) - automatically installed by install scripts if not present
- `cargo-watch` (for development mode, installed automatically)
- SSH client
- Tailscale (optional, for Tailscale connections) - can be installed via `hal tailscale install`

