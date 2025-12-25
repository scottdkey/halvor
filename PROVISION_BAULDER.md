# Provisioning Baulder and Joining to K3s Cluster

Since frigg is already set up with k3s and the agent is running, we'll join baulder to the cluster.

## Step 1: Join Baulder to the Cluster

The join command will automatically:
1. Build and push halvor to experimental (if in dev mode)
2. Install halvor on baulder
3. Join baulder to k3s as a control plane node
4. Set up halvor agent service on baulder

Run:

```bash
halvor k3s join baulder --server=<frigg_tailscale_hostname> --control-plane
```

Or if you have `K3S_TOKEN` in your environment:

```bash
halvor k3s join baulder --server=<frigg_tailscale_hostname> --control-plane
```

**To get frigg's Tailscale hostname:**
```bash
# Check frigg's Tailscale hostname
halvor ssh frigg "tailscale status | grep frigg"
```

**Example:**
```bash
# If frigg's Tailscale hostname is frigg.ts.net
halvor k3s join baulder --server=frigg.ts.net --control-plane
```

## Step 2: Verify the Join

After joining, verify the cluster status:

```bash
# Check cluster status from frigg
halvor k3s status -H frigg

# Or check from baulder
halvor k3s status -H baulder
```

You should see both frigg and baulder listed as control plane nodes.

## Step 3: Verify Agent Services

Check that the halvor agent is running on both nodes:

```bash
# Check agent on frigg
halvor ssh frigg "systemctl status halvor-agent"

# Check agent on baulder  
halvor ssh baulder "systemctl status halvor-agent"
```

Both agents should be running on port 13500 (accessible over Tailscale).

## Step 4: Discover Agents (Mesh Network)

Once both agents are running, you can discover them:

```bash
# Discover agents from frigg
halvor agent discover -H frigg

# Discover agents from baulder
halvor agent discover -H baulder
```

Both nodes should discover each other via Tailscale.

## Troubleshooting

### Join Fails

If the join fails:

```bash
# Check k3s logs on baulder
halvor ssh baulder "journalctl -u k3s -n 50"

# Verify Tailscale connectivity
halvor ssh frigg "ping -c 1 baulder.ts.net"
halvor ssh baulder "ping -c 1 frigg.ts.net"
```

### Agent Service Not Starting

If the agent service fails to start:

```bash
# Check logs on baulder
halvor ssh baulder "journalctl -u halvor-agent -n 50"

# Manually start agent for testing
halvor ssh baulder "halvor agent start --port 13500"
```

