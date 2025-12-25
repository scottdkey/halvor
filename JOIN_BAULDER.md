# Joining Baulder to K3s Cluster and Mesh Network

## Prerequisites

1. **Frigg is initialized** as the primary control plane node
2. **Tailscale is configured** on both frigg and baulder
3. **SSH access** to baulder is configured in your `.env` file

## Step 1: Get Cluster Join Information

First, get the server address and token from frigg:

```bash
# Get join information from frigg
halvor k3s status -H frigg
```

Or get it programmatically:

```bash
# The join command will automatically fetch this, but you can also get it manually:
# Server address will be frigg's Tailscale hostname (e.g., frigg.ts.net)
# Token is in K3S_TOKEN environment variable (from 1Password/direnv)
```

## Step 2: Join Baulder to the Cluster

Run the join command. This will automatically:

1. **Build and push halvor to experimental** (if in development mode)
2. **Install halvor** on baulder from the experimental release
3. **Join baulder** to the k3s cluster as a control plane node
4. **Set up halvor agent service** on baulder (port 13500 for agent, port 13000 for web UI if available)

```bash
# Join baulder as a control plane node
halvor k3s join baulder --server=<frigg_tailscale_hostname> --token=<K3S_TOKEN> --control-plane
```

Or if `K3S_TOKEN` is in your environment:

```bash
halvor k3s join baulder --server=<frigg_tailscale_hostname> --control-plane
```

**Example:**
```bash
# If frigg's Tailscale hostname is frigg.ts.net
halvor k3s join baulder --server=frigg.ts.net --control-plane
```

## Step 3: Verify the Join

After joining, verify the cluster status:

```bash
# Check cluster status from frigg
halvor k3s status -H frigg

# Or check from baulder
halvor k3s status -H baulder
```

You should see both frigg and baulder listed as control plane nodes.

## Step 4: Verify Agent Services

Check that the halvor agent is running on both nodes:

```bash
# Check agent on frigg
halvor ssh frigg "systemctl status halvor-agent"

# Check agent on baulder  
halvor ssh baulder "systemctl status halvor-agent"
```

Both agents should be running on port 13500 (accessible over Tailscale).

## Step 5: Discover Agents (Mesh Network)

Once both agents are running, you can discover them:

```bash
# Discover agents from frigg
halvor agent discover -H frigg

# Discover agents from baulder
halvor agent discover -H baulder
```

Both nodes should discover each other via Tailscale.

## Troubleshooting

### Agent Service Not Starting

If the agent service fails to start:

```bash
# Check logs on baulder
halvor ssh baulder "journalctl -u halvor-agent -n 50"

# Manually start agent for testing
halvor ssh baulder "halvor agent start --port 13500"
```

### Build/Push to Experimental Fails

If the build/push step fails, you can manually build and push:

```bash
# Build and push to experimental
halvor build cli --experimental
```

Then retry the join command.

### K3s Join Fails

If k3s join fails:

```bash
# Check k3s logs on baulder
halvor ssh baulder "journalctl -u k3s -n 50"

# Verify Tailscale connectivity
halvor ssh frigg "ping -c 1 baulder.ts.net"
halvor ssh baulder "ping -c 1 frigg.ts.net"
```

## Next Steps

After baulder is joined:

1. **Set up agent service on frigg** (if not already done):
   ```bash
   halvor ssh frigg "halvor agent start --port 13500 --daemon"
   # Or set up as systemd service manually
   ```

2. **Test mesh network communication**:
   ```bash
   halvor agent discover -H frigg
   halvor agent discover -H baulder
   ```

3. **Join additional nodes** (e.g., oak) using the same process.

