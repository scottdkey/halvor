# Installation Scripts

This directory contains installation scripts for installing `halvor` from GitHub or Gitea releases.

## Installation Scripts

### `install.sh` (Linux/macOS)

Downloads and installs the `halvor` CLI tool from GitHub releases.

**Usage:**

```bash
# Install from GitHub
curl -fsSL https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.sh | bash

# Or install from Gitea (if hosted)
curl -fsSL https://gitea.scottkey.me/scottkey/halvor/raw/branch/main/scripts/install.sh | bash
```

The script automatically:

- Detects your platform (OS and architecture)
- Downloads the correct pre-built binary from GitHub/Gitea releases
- Installs to `/usr/local/bin` (Linux/macOS)
- Sets up PATH if needed

### `install.ps1` (Windows)

Downloads and installs the `halvor` CLI tool from GitHub releases.

**Usage:**

```powershell
# Install from GitHub
irm https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.ps1 | iex

# Or install from Gitea (if hosted)
irm https://gitea.scottkey.me/scottkey/halvor/raw/branch/main/scripts/install.ps1 | iex
```

The script automatically:

- Detects your platform (OS and architecture)
- Downloads the correct pre-built binary from GitHub/Gitea releases
- Installs to `~/.local/bin` (Windows)
- Sets up PATH if needed

## Installation Options

Both scripts support environment variables to customize installation:

```bash
# Use experimental channel
GITHUB_REPO=scottdkey/halvor EXPERIMENTAL=true curl -fsSL https://raw.githubusercontent.com/scottdkey/halvor/main/scripts/install.sh | bash

# Install from Gitea instead of GitHub
GITEA_URL=https://gitea.scottkey.me/scottkey/halvor curl -fsSL https://gitea.scottkey.me/scottkey/halvor/raw/branch/main/scripts/install.sh | bash
```

## Note

All other setup and management tasks should be done using the `halvor` CLI itself. The CLI provides commands for:
- Cluster setup: `halvor k3s setup`
- SMB mounts: `halvor smb`
- Service provisioning: `halvor provision`
- And much more

See `halvor --help` for all available commands.
