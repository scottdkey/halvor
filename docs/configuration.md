# Configuration Guide

## Overview

Halvor uses environment variables loaded from a `.env` file. The `.env` file is typically managed via **direnv** and **1Password** for secure secret management.

## Environment File Setup

### Using direnv + 1Password (Recommended)

1. **Setup direnv** (if not already installed):
   ```bash
   # macOS
   brew install direnv
   
   # Linux
   sudo apt install direnv
   ```

2. **Configure `.envrc`** to load secrets from 1Password:
   ```bash
   # Copy example (if it exists)
   cp .envrc.example .envrc
   
   # Edit .envrc with your 1Password vault reference
   # Example .envrc content:
   #   eval $(op signin)
   #   export $(op inject -i .env.template)
   
   # If you have multiple 1Password accounts, use a specific account:
   #   eval $(op signin --account <account-url-or-uuid>)
   #   # Or list accounts first: op account list
   #   # Then use the full account URL or UUID
   
   # Then allow direnv
   direnv allow
   ```
   
   **Note:** If you get "found multiple accounts for filter" error:
   - List your accounts: `op account list`
   - Use the full account URL or UUID instead of a short name
   - Example: `eval $(op signin --account my.1password.com)` or `eval $(op signin --account <uuid>)`

3. **Environment variables are automatically loaded** when you enter the directory.

### Manual .env File

Alternatively, create a `.env` file manually:

```bash
# Create example file
halvor config env

# Or manually create .env in your project directory
```

## Configuration Format

### Host Configurations

Format: `HOST_<HOSTNAME>_<FIELD>=<value>`

```bash
# Tailscale base domain
TAILNET_BASE="ts.net"

# Host IP addresses
HOST_FRIGG_IP="10.10.10.10"
HOST_BAULDER_IP="10.10.10.11"

# Host hostnames (typically Tailscale hostnames)
HOST_FRIGG_HOSTNAME="frigg.ts.net"
HOST_BAULDER_HOSTNAME="baulder.ts.net"

# Backup paths (optional)
HOST_FRIGG_BACKUP_PATH="/mnt/smb/maple/backups/frigg"
HOST_BAULDER_BACKUP_PATH="/mnt/smb/maple/backups/baulder"
```

### SMB Server Configuration

Format: `SMB_<SERVERNAME>_<FIELD>=<value>`

**Required fields:**
- `HOST` - SMB server IP address or hostname
- `SHARES` - Comma-separated list of share names

**Optional fields:**
- `USERNAME` - SMB username
- `PASSWORD` - SMB password
- `OPTIONS` - Additional mount options

**Example:**
```bash
# SMB server configuration
SMB_MAPLE_HOST="10.10.10.130"
SMB_MAPLE_SHARES="backups,data,halvor"
SMB_MAPLE_USERNAME="skey"
SMB_MAPLE_PASSWORD="your-password"
SMB_MAPLE_OPTIONS="vers=3.0"  # Optional mount options

# Multiple SMB servers
SMB_WILLOW_HOST="10.10.10.10"
SMB_WILLOW_SHARES="backups,data,halvor"
SMB_WILLOW_USERNAME="skey"
SMB_WILLOW_PASSWORD="your-password"
```

### Nginx Proxy Manager Configuration

For NPM automation, add these to your `.env` file:

```bash
NPM_URL="https://frigg:81"  # Optional, defaults to https://{hostname}:81
NPM_USERNAME="admin@example.com"
NPM_PASSWORD="your-password"
```

## Managing Configuration

### View Current Configuration

```bash
# List all configuration
halvor config list

# Show configuration with verbose output (including passwords)
halvor config --verbose
```

### Initialize Configuration

```bash
# Interactive configuration setup
halvor config init
```

### Create Example .env File

```bash
# Generate .env.example template
halvor config env
```

### Create SMB Server Configuration

```bash
# Interactive SMB server setup
halvor config create smb

# Or specify server name
halvor config create smb maple
```

### Set Environment File Path

```bash
halvor config set-env /path/to/.env
```

### Host-Specific Configuration

```bash
# Set IP address for a host
halvor config ip <hostname> <ip-address>

# Set hostname for a host
halvor config hostname <hostname> <hostname-value>

# Set backup path for a host
halvor config backup-path <hostname> <backup-path>
```

## Configuration Storage

- **Primary**: Environment variables in `.env` file (loaded from 1Password via direnv)
- **Secondary**: SQLite database at `~/.hal/halvor.db` (for runtime data)

Configuration is automatically loaded from the `.env` file. The database is used for storing runtime state and encrypted data.

## See Also

- [Setup Guide](setup.md) - Initial project setup
- [Usage Guide](usage.md) - Common commands
- [CLI Commands Reference](generated/cli-commands.md) - Complete command documentation
