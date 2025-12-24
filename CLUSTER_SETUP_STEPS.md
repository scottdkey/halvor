# K3s Cluster Setup Steps

## Quick Setup (Recommended)

Use the integrated setup command to set up your entire cluster:

```bash
halvor k3s setup --primary frigg --nodes baulder,oak
```

This single command will:
1. Setup SMB mounts on all deployment nodes (frigg, baulder)
2. Initialize frigg as primary control plane
3. Join baulder and oak as additional control plane nodes
4. Verify cluster health

### Options

```bash
# Skip SMB mount setup (if already configured)
halvor k3s setup --primary frigg --nodes baulder,oak --skip-smb

# Specify which nodes need SMB mounts (defaults to all deployment nodes)
halvor k3s setup --primary frigg --nodes baulder,oak --smb-nodes frigg,baulder

# Skip cluster verification after setup
halvor k3s setup --primary frigg --nodes baulder,oak --skip-verify
```

## Manual Setup (Step-by-Step)

If you prefer to set up the cluster manually:

### Step 1: Setup SMB Mounts

Setup SMB mounts on the deployment nodes (frigg and baulder):

```bash
# Setup SMB mounts on frigg
halvor smb -H frigg

# Setup SMB mounts on baulder
halvor smb -H baulder
```

This will mount the SMB shares needed for persistent storage:
- `/mnt/smb/maple/backups` - Backups (primary)
- `/mnt/smb/maple/data` - Data (primary, shared)
- `/mnt/smb/maple/halvor` - Halvor container data (primary)
- `/mnt/smb/willow/backups` - Backups (fallback)
- `/mnt/smb/willow/data` - Data (fallback, shared)
- `/mnt/smb/willow/halvor` - Halvor container data (fallback)

### Step 2: Provision frigg as Primary Control Plane

Initialize frigg as the first control plane node:

```bash
halvor provision -H frigg -y --cluster-role init
```

This will:
- Install k3s on frigg
- Initialize the cluster with embedded etcd
- Generate a cluster token (stored in K3S_TOKEN env var)
- Configure Tailscale networking

### Step 3: Provision baulder as HA Control Plane

Join baulder to the cluster as a control plane node:

```bash
# Get frigg's Tailscale hostname (replace ts.net with your tailnet base)
FRIGG_SERVER="frigg.ts.net"  # or use: frigg.$(halvor config show | grep TAILNET_BASE | cut -d'=' -f2 | tr -d ' ')

halvor provision -H baulder -y --cluster-role control-plane --cluster-server "$FRIGG_SERVER"
```

This will:
- Install k3s on baulder
- Join to the cluster as a control plane node
- Configure HA with etcd

### Step 4: Provision oak as Tiebreaker

Join oak to the cluster as a control plane node (tiebreaker):

```bash
halvor provision -H oak -y --cluster-role control-plane --cluster-server "$FRIGG_SERVER"
```

### Step 5: Verify Cluster

Check cluster status:

```bash
# Check status from frigg
halvor k3s status -H frigg

# Verify HA cluster health
halvor k3s verify --nodes frigg,baulder,oak -H frigg
```

You should see all three nodes (frigg, baulder, oak) in the cluster.

## Step 6: Deploy Infrastructure

Once the cluster is verified, deploy the infrastructure:

```bash
# Deploy SMB storage (creates PersistentVolumes)
halvor install smb-storage --helm

# Deploy Gitea
halvor install gitea --helm

# Deploy other services as needed
halvor install pia-vpn --helm
halvor install halvor-server --helm
```

## Troubleshooting

### Cluster not reachable
If you get connection errors:
```bash
# Check Tailscale connectivity
halvor ssh frigg "ping -c 1 baulder.ts.net"
halvor ssh frigg "ping -c 1 oak.ts.net"

# Verify k3s service is running
halvor ssh frigg "sudo systemctl status k3s"
```

### Node not joining
If a node fails to join:
```bash
# Check k3s service logs
halvor ssh baulder "sudo journalctl -u k3s -n 50"

# Verify token is correct
echo $K3S_TOKEN  # Should match the token from frigg

# Re-provision if needed (will clean up and retry)
halvor provision -H baulder -y --cluster-role control-plane --cluster-server "$FRIGG_SERVER"
```

### SMB mounts not working
If SMB mounts fail:
```bash
# Check SMB server connectivity
halvor ssh frigg "ping -c 1 maple"
halvor ssh frigg "ping -c 1 willow"

# Re-setup SMB mounts
halvor smb -H frigg
halvor smb -H baulder
```

