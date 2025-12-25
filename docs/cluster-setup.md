# K3s Cluster Setup Guide

This guide provides comprehensive instructions for setting up a K3s Kubernetes cluster using halvor.

## Prerequisites

- Servers configured: frigg (primary), baulder (HA control plane), oak (tiebreaker)
- SMB servers configured: maple (primary), willow (fallback)
- Environment variables loaded from 1Password via direnv
- SSH access configured to all nodes
- Tailscale installed and configured on all nodes

## Manual Setup (Step-by-Step)

If you prefer to set up the cluster manually:

### Step 1: Setup SMB Mounts

Setup SMB mounts on the deployment nodes (frigg and baulder):

```bash
# Setup SMB mounts on frigg
halvor install smb -H frigg

# Setup SMB mounts on baulder
halvor install smb -H baulder
```

This will mount the SMB shares needed for persistent storage:
- `/mnt/smb/maple/backups` - Backups (primary)
- `/mnt/smb/maple/data` - Data (primary, shared)
- `/mnt/smb/maple/halvor` - Halvor container data (primary)
- `/mnt/smb/willow/backups` - Backups (fallback)
- `/mnt/smb/willow/data` - Data (fallback, shared)
- `/mnt/smb/willow/halvor` - Halvor container data (fallback)

### Step 2: Initialize frigg as Primary Control Plane

Initialize frigg as the first control plane node:

```bash
halvor init -H frigg
```

This will:
- Install k3s on frigg
- Initialize the cluster with embedded etcd
- Generate a cluster token (stored in K3S_TOKEN env var)
- Configure Tailscale networking

### Step 3: Join baulder as HA Control Plane

Join baulder to the cluster as a control plane node:

```bash
# Get frigg's Tailscale hostname (replace ts.net with your tailnet base)
FRIGG_SERVER="frigg.ts.net"  # Use the Tailscale hostname from your .env file (HOST_FRIGG_HOSTNAME)

halvor join -H baulder --server="$FRIGG_SERVER" --control-plane
```

This will:
- Install k3s on baulder
- Join to the cluster as a control plane node
- Configure HA with etcd

### Step 4: Join oak as Tiebreaker

Join oak to the cluster as a control plane node (tiebreaker):

```bash
halvor join -H oak --server="$FRIGG_SERVER" --control-plane
```

### Alternative: Using the Join Command

You can also use the `halvor join` command to join nodes:

#### Remote Join (from another machine)

Run from a different machine (e.g., frigg) to join a remote node (e.g., baulder):

```bash
# From frigg, join baulder to the cluster
halvor join -H baulder --server=frigg --control-plane

# Auto-detect primary node
halvor join -H baulder --control-plane
```

#### Local Join (on the target machine)

Run directly on the target machine (e.g., baulder) to join itself:

```bash
# From baulder, join itself to the cluster
halvor join --server=frigg --control-plane

# Auto-detect primary node
halvor join --control-plane
```

**How it works:**
- The `-H baulder` flag specifies the target node for remote operations
- Without `-H`, the command runs on localhost
- The executor automatically detects whether this is a remote or local operation
- Auto-detection checks if the current machine has K3s running as a control plane

### Step 5: Verify Cluster

Check cluster status:

```bash
# Check status from frigg
halvor status k3s -H frigg

# Check Helm releases
halvor status helm -H frigg
```

You should see all three nodes (frigg, baulder, oak) in the cluster.

### Step 6: Deploy Infrastructure

Once the cluster is verified, deploy the infrastructure:

```bash
# Deploy SMB storage (creates PersistentVolumes)
# The CLI automatically detects these are Helm charts - no --helm flag needed
halvor install smb-storage -H frigg

# Deploy Gitea
halvor install gitea -H frigg

# Deploy other services as needed
halvor install pia-vpn -H frigg
halvor install halvor-server -H frigg
```

**Important**: Always use `-H frigg` (or `-H baulder`) for Helm chart deployments to ensure they go to the cluster, not your local machine.

## Token Handling

The cluster token can be provided in three ways:

1. **Environment variable**: `K3S_TOKEN` (loaded from 1Password via direnv)
2. **Command line**: `--token=<token>`
3. **Auto-fetch**: Fetched from the primary node if not provided

## Troubleshooting

### Cluster not reachable

If you get connection errors:

```bash
# Check Tailscale connectivity
ssh frigg "ping -c 1 baulder.ts.net"
ssh frigg "ping -c 1 oak.ts.net"

# Verify k3s service is running
ssh frigg "sudo systemctl status k3s"
```

### Node not joining

If a node fails to join:

```bash
# Check k3s service logs
ssh baulder "sudo journalctl -u k3s -n 50"

# Verify token is correct
echo $K3S_TOKEN  # Should match the token from frigg

# Re-join if needed (will clean up and retry)
halvor join -H baulder --server="$FRIGG_SERVER" --control-plane
```

### SMB mounts not working

If SMB mounts fail:

```bash
# Check SMB server connectivity
ssh frigg "ping -c 1 maple"
ssh frigg "ping -c 1 willow"

# Re-setup SMB mounts
halvor install smb -H frigg
halvor install smb -H baulder
```

### "Host not found in config"

Ensure the target host is configured in your `.env` file:

```
HOST_BAULDER_IP="100.x.x.x"
HOST_BAULDER_HOSTNAME="baulder.bombay-pinecone.ts.net"
```

### "Server address not provided and could not auto-detect"

Specify the primary node explicitly:

```bash
halvor join -H baulder --server=frigg --control-plane
```

### Service Issues

If services fail to start:

```bash
# Check service logs
ssh baulder "journalctl -u <service-name> -n 50"

# Check cluster pods
kubectl get pods -A
kubectl logs -f <pod-name> -n <namespace>
```

## Next Steps

After the cluster is set up:

1. **Deploy Additional Services**: Use `halvor install <service> -H frigg`
2. **Monitor Deployments**: `kubectl get pods -A`
3. **View Logs**: `kubectl logs -f deployment/<name> -n default`
4. **Setup CI/CD**: Configure Gitea Actions workflows for automated deployments

## Additional Resources

- [Configuration Guide](configuration.md) - Setting up your environment
- [Usage Guide](usage.md) - Common commands and operations
- [Agent Architecture](agent-architecture.md) - Understanding the agent mesh network

