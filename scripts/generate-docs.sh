#!/bin/bash

# Generate documentation for halvor
# This script generates:
# - docs/generated/cli-commands.md - Complete CLI command reference
# - docs/generated/docker-containers.md - Available Docker containers
# - docs/generated/helm-charts.md - Available Helm charts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_DIR="$PROJECT_ROOT/docs/generated"

# Ensure docs directory exists
mkdir -p "$DOCS_DIR"

# Check if halvor is available
if ! command -v halvor >/dev/null 2>&1; then
    echo "Error: halvor command not found. Please build halvor first:"
    echo "  cargo build --release"
    echo "  cargo install --path . --bin halvor --force"
    exit 1
fi

echo "Generating documentation..."

# Generate CLI Commands Reference
echo "  - Generating cli-commands.md..."
cat > "$DOCS_DIR/cli-commands.md" << 'EOF'
# CLI Commands Reference

This document is auto-generated from the halvor CLI. For the most up-to-date information, run `halvor --help` or `halvor <command> --help`.

EOF

# Get main help
halvor --help >> "$DOCS_DIR/cli-commands.md" 2>&1 || true

# Add subcommand help sections
echo "" >> "$DOCS_DIR/cli-commands.md"
echo "## Subcommands" >> "$DOCS_DIR/cli-commands.md"
echo "" >> "$DOCS_DIR/cli-commands.md"

# Get help for each subcommand
for cmd in backup restore sync list install uninstall update init join status configure config db build dev generate; do
    echo "" >> "$DOCS_DIR/cli-commands.md"
    echo "### \`halvor $cmd\`" >> "$DOCS_DIR/cli-commands.md"
    echo "" >> "$DOCS_DIR/cli-commands.md"
    echo '```' >> "$DOCS_DIR/cli-commands.md"
    halvor "$cmd" --help 2>&1 | head -100 >> "$DOCS_DIR/cli-commands.md" || true
    echo '```' >> "$DOCS_DIR/cli-commands.md"
done

# Generate Docker Containers Reference
echo "  - Generating docker-containers.md..."
cat > "$DOCS_DIR/docker-containers.md" << 'EOF'
# Docker Containers

This document lists all Docker containers that can be built using halvor.

## Available Containers

EOF

# Extract container info from code (currently only pia-vpn)
cat >> "$DOCS_DIR/docker-containers.md" << 'EOF'
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

EOF

# Generate Helm Charts Reference
echo "  - Generating helm-charts.md..."
cat > "$DOCS_DIR/helm-charts.md" << 'EOF'
# Helm Charts

This document lists all Helm charts available for installation via halvor.

## Available Helm Charts

Helm charts are automatically detected by halvor - no `--helm` flag is needed. Simply use:

```bash
halvor install <chart-name> -H <hostname>
```

EOF

# Get list of apps and filter for Helm charts
echo "" >> "$DOCS_DIR/helm-charts.md"
echo "## Charts" >> "$DOCS_DIR/helm-charts.md"
echo "" >> "$DOCS_DIR/helm-charts.md"

# Use halvor install --list to get apps, then filter for Helm charts
halvor install --list 2>&1 | grep -A 1000 "Helm Charts:" >> "$DOCS_DIR/helm-charts.md" || {
    # Fallback: manually list known Helm charts
    cat >> "$DOCS_DIR/helm-charts.md" << 'EOF'
### portainer

**Namespace**: `default`  
**Description**: Portainer CE/BE/Agent - Container management UI (use deploymentType: ce/be/agent)

**Installation**:
```bash
halvor install portainer -H <hostname>
```

### nginx-proxy-manager

**Namespace**: `default`  
**Aliases**: `npm`, `proxy`  
**Description**: Reverse proxy with SSL

**Installation**:
```bash
halvor install nginx-proxy-manager -H <hostname>
# or
halvor install npm -H <hostname>
```

### traefik-public

**Namespace**: `traefik`  
**Aliases**: `traefik-pub`, `traefik-dev`  
**Description**: Public Traefik reverse proxy (PUBLIC_DOMAIN from 1Password)

**Installation**:
```bash
halvor install traefik-public -H <hostname>
```

### traefik-private

**Namespace**: `traefik`  
**Aliases**: `traefik-priv`, `traefik-me`  
**Description**: Private Traefik reverse proxy (PRIVATE_DOMAIN from 1Password, local/Tailnet only)

**Installation**:
```bash
halvor install traefik-private -H <hostname>
```

### gitea

**Namespace**: `default`  
**Description**: Gitea Git hosting service

**Installation**:
```bash
halvor install gitea -H <hostname>
```

### pia-vpn

**Namespace**: `default`  
**Aliases**: `pia`, `vpn`  
**Description**: Private Internet Access VPN proxy service

**Installation**:
```bash
halvor install pia-vpn -H <hostname>
```

For more information, see the [VPN Guide](../vpn.md).

### sonarr

**Namespace**: `default`  
**Description**: Sonarr - TV show collection manager

**Installation**:
```bash
halvor install sonarr -H <hostname>
```

### radarr

**Namespace**: `default`  
**Description**: Radarr - Movie collection manager

**Installation**:
```bash
halvor install radarr -H <hostname>
```

### radarr (multiple instances)

**Namespace**: `default`  
**Description**: Radarr - Movie collection manager. Deploy multiple instances with different release names using the `--name` flag.

**Installation**:
```bash
# Default instance
halvor install radarr -H <hostname>

# 4K instance
halvor install radarr --name radarr-4k -H <hostname>

# Anime instance
halvor install radarr --name radarr-anime -H <hostname>
```

### prowlarr

**Namespace**: `default`  
**Description**: Prowlarr - Indexer manager

**Installation**:
```bash
halvor install prowlarr -H <hostname>
```

### bazarr

**Namespace**: `default`  
**Description**: Bazarr - Subtitle manager

**Installation**:
```bash
halvor install bazarr -H <hostname>
```

### sabnzbd

**Namespace**: `default`  
**Description**: SABnzbd - Usenet downloader

**Installation**:
```bash
halvor install sabnzbd -H <hostname>
```

### qbittorrent

**Namespace**: `default`  
**Description**: qBittorrent - BitTorrent client

**Installation**:
```bash
halvor install qbittorrent -H <hostname>
```

### smb-storage

**Namespace**: `default`  
**Description**: SMB storage provisioner for Kubernetes

**Installation**:
```bash
halvor install smb-storage -H <hostname>
```

**Note**: SMB mounts should be set up on cluster nodes using `halvor install smb -H <hostname>` before deploying this chart.

### halvor-server

**Namespace**: `default`  
**Description**: Halvor agent server

**Installation**:
```bash
halvor install halvor-server -H <hostname>
```

EOF
}

echo "" >> "$DOCS_DIR/helm-charts.md"
echo "## Notes" >> "$DOCS_DIR/helm-charts.md"
echo "" >> "$DOCS_DIR/helm-charts.md"
echo "- All Helm charts are automatically detected - no \`--helm\` flag needed" >> "$DOCS_DIR/helm-charts.md"
echo "- The CLI validates cluster availability before installing Helm charts" >> "$DOCS_DIR/helm-charts.md"
echo "- Charts default to the \`frigg\` hostname if no \`-H\` option is provided" >> "$DOCS_DIR/helm-charts.md"
echo "- Use \`halvor install --list\` to see all available apps" >> "$DOCS_DIR/helm-charts.md"

echo "✓ Documentation generated in docs/generated/"

