# Usage Guide

This guide provides quick examples for common tasks. For complete command reference, see the [Auto-Generated CLI Commands Documentation](generated/cli-commands.md).

## Quick Examples

### Install Services

```bash
# List all available apps
halvor install --list

# Install platform tools
halvor install docker -H frigg
halvor install tailscale -H frigg
halvor install smb -H frigg

# Install Helm charts (deploys to Kubernetes cluster)
# The CLI automatically detects Helm charts - no --helm flag needed
halvor install portainer -H frigg
halvor install gitea -H frigg
```

### Backup and Restore

```bash
# Backup all services interactively
halvor backup -H frigg

# Backup a specific service
halvor backup portainer -H frigg

# List available backups
halvor backup -H frigg --list

# Restore a service
halvor restore portainer -H frigg
```

### Cluster Management

```bash
# Initialize cluster
halvor init -H frigg

# Join nodes
halvor join -H baulder --server=frigg --control-plane

# Check cluster status
halvor status k3s -H frigg
```

For complete command documentation with all options, see [CLI Commands Reference](generated/cli-commands.md).

