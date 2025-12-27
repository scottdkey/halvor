# Docker Containers

This document lists all Docker containers that can be built using halvor.

## Available Containers

### pia-vpn

**Image**: `ghcr.io/scottdkey/pia-vpn`  
**Build Directory**: `vpn-container/`  
**Description**: Private Internet Access VPN container with Rust-based entrypoint

**Build Command**:
```bash
halvor build pia-vpn [--no-cache] [--push] [--release]
```

**Options**:
- `--no-cache` - Build without using cache
- `--push` - Push to GitHub Container Registry (experimental tag)
- `--release` - Push as release (latest tag instead of experimental)

**Usage**:
```bash
# Build locally
halvor build pia-vpn

# Build and push to registry
halvor build pia-vpn --push

# Build and push as release
halvor build pia-vpn --push --release
```

For more information, see the [VPN Guide](../vpn.md).

