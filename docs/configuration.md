# Configuration Guide

## Initial Setup

When `hal` is installed globally, you need to configure the location of your `.env` file:

```bash
hal config init
```

This will:

- Prompt you for the path to your `.env` file
- Store the configuration in `~/.config/hal/config.toml`
- Allow `hal` to work from any directory

## Environment File

1. Copy `.env.example` to `.env` (or create your own):

   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your host configurations:

   ```bash
   # Tailscale configuration
   TAILNET_BASE="ts.net"

   # Host configurations
   HOST_FRIGG_IP="10.10.10.10"
   HOST_FRIGG_HOSTNAME="frigg.ts.net"

   # SSH host configurations (for setup-ssh-hosts.sh)
   SSH_MAPLE_HOST="10.10.10.130"
   SSH_MAPLE_USER="skey"
   SSH_FRIGG_HOST="10.10.10.10"
   SSH_FRIGG_USER="skey"
   ```

## Managing Configuration

**View current configuration:**

```bash
hal config show
```

**Set environment file path:**

```bash
hal config set-env /path/to/.env
```

**Re-initialize configuration (interactive):**

```bash
hal config init
```

## Nginx Proxy Manager Configuration

For NPM automation, add these to your `.env` file:

```bash
NPM_URL="https://frigg:81"  # Optional, defaults to https://{hostname}:81
NPM_USERNAME="admin@example.com"
NPM_PASSWORD="your-password"
```

## SMB Configuration

For SMB mount automation, configure SMB servers in your `.env`:

```bash
SMB_MAPLE_HOST="10.10.10.130"
SMB_MAPLE_USER="username"
SMB_MAPLE_PASSWORD="password"
SMB_MAPLE_DOMAIN="WORKGROUP"  # Optional
```
