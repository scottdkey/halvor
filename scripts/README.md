# Scripts

This directory contains utility scripts for setting up and managing your homelab.

## Installation Scripts

### `install.sh` / `install.ps1`

Downloads and installs the `hal` CLI tool from GitHub releases.

**Usage:**

```bash
# Linux/macOS
curl -fsSL https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/install.sh | bash

# Windows
irm https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/install.ps1 | iex
```

The scripts automatically:

- Detect your platform (OS and architecture)
- Download the correct pre-built binary from GitHub releases
- Install to `/usr/local/bin` (Linux/macOS) or `~/.local/bin` (Windows)
- Set up PATH if needed

## SSH Setup Scripts

### `setup-ssh-hosts.sh`

Configures SSH hosts in `~/.ssh/config` from your `.env` file.

**Usage:**

```bash
./scripts/setup-ssh-hosts.sh
```

**Configuration in `.env`:**

```bash
SSH_MAPLE_HOST="10.10.10.130"
SSH_MAPLE_USER="test-user"
SSH_MAPLE_PORT="22"

SSH_BELLEROPHON_HOST="10.10.10.14"
SSH_BELLEROPHON_USER="username"
```

This creates SSH config entries that allow you to connect with:

```bash
ssh maple
ssh bellerophon
```

### `setup-ssh-keys.sh`

Sets up SSH key authentication on remote hosts. Uses password authentication initially, then enables
key-based auth.

**Usage:**

```bash
./scripts/setup-ssh-keys.sh <hostname> [username]
```

**Example:**

```bash
./scripts/setup-ssh-keys.sh maple
```

This will:

1. Check if SSH key is already installed
2. If not, prompt for password once
3. Copy your SSH public key to the remote host
4. Enable passwordless SSH connections

After running this, you can connect without a password:

```bash
ssh maple
```

## VPN Scripts

### `setup-vpn-firewall.sh`

Sets up firewall rules for VPN containers. See the script for details.

## Running Scripts Remotely

You can also run scripts directly from GitHub:

```bash
# Setup SSH hosts
curl -fsSL https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/setup-ssh-hosts.sh | bash

# Setup SSH keys
curl -fsSL https://raw.githubusercontent.com/scottdkey/homelab/main/scripts/setup-ssh-keys.sh | bash -s <hostname>
```
