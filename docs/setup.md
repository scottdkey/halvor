# Setup Guide

## Initial Configuration

1. **Configure halvor**: 
   ```bash
   halvor config init
   ```
   This sets up the path to your `.env` file and initializes configuration.

2. **Setup SSH hosts**: 
   ```bash
   ./scripts/setup-ssh-hosts.sh
   ```
   This reads SSH host configurations from your `.env` file and adds them to `~/.ssh/config`.

3. **Setup SSH keys**: 
   ```bash
   ./scripts/setup-ssh-keys.sh <hostname>
   ```
   Copies your SSH public key to the remote host (one-time password required).

## Environment Configuration

Add host configurations to your `.env` file:

```bash
HOST_FRIGG_IP="10.10.10.10"
HOST_FRIGG_HOSTNAME="frigg.ts.net"
HOST_FRIGG_USER="skey"

HOST_BAULDER_IP="10.10.10.11"
HOST_BAULDER_HOSTNAME="baulder.ts.net"
HOST_BAULDER_USER="skey"
```

For detailed configuration options, see the [Configuration Guide](configuration.md).

## Next Steps

- **Install platform tools**: See [Cluster Setup Guide](cluster-setup.md) for installing Docker, Tailscale, and SMB mounts
- **Initialize cluster**: See [Cluster Setup Guide](cluster-setup.md) for K3s cluster setup
- **Install services**: See [CLI Commands Reference](generated/cli-commands.md) for `halvor install` command
