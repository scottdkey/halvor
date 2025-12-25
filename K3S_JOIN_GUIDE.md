# K3s Join Command Guide

The `halvor join` command supports two execution modes:

## 1. Remote Join (from another machine)

Run from a different machine (e.g., frigg) to join a remote node (e.g., baulder):

```bash
# From frigg, join baulder to the cluster
halvor join -H baulder --server=frigg --control-plane

# Auto-detect primary node
halvor join -H baulder --control-plane
```

**How it works:**
- The `-H baulder` flag specifies the target node
- The command creates an SSH connection to baulder
- All operations (K3s installation, Tailscale setup, etc.) are performed remotely via SSH
- The executor automatically detects this is a remote operation

## 2. Local Join (on the target machine)

Run directly on the target machine (e.g., baulder) to join itself:

```bash
# From baulder, join itself to the cluster
halvor join --server=frigg --control-plane

# Auto-detect primary node
halvor join --control-plane
```

**How it works:**
- No `-H` flag means the command runs on localhost
- The executor detects this is a local operation
- All operations are performed directly on the current machine
- No SSH connection is needed

## Auto-Detection

The command can auto-detect the primary control plane node:

1. **First**: Checks if the current machine (where the command is running) has K3s running as a control plane
2. **Second**: Checks other configured hosts that are local (by IP comparison) to avoid SSH prompts
3. **If not found**: Requires `--server` to be specified

## Examples

### Scenario 1: Join baulder from frigg (remote)

```bash
# On frigg
halvor join -H baulder --server=frigg --control-plane
```

### Scenario 2: Join baulder from baulder (local)

```bash
# On baulder
halvor join --server=frigg --control-plane
```

### Scenario 3: Auto-detect primary (from baulder)

```bash
# On baulder - will auto-detect frigg as primary
halvor join --control-plane
```

## Token Handling

The cluster token can be provided in three ways:

1. **Environment variable**: `K3S_TOKEN` (loaded from 1Password via direnv)
2. **Command line**: `--token=<token>`
3. **Auto-fetch**: Fetched from the primary node if not provided

## Troubleshooting

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

### "Failed to create executor"

Check that:
- The host is in your `.env` configuration
- SSH access is configured (keys or password)
- Tailscale is running if using Tailscale hostnames

