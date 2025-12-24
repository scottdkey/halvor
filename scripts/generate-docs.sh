#!/bin/bash
# Generate documentation from CLI commands, Docker containers, and Helm charts

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DOCS_DIR="$REPO_ROOT/docs"
GENERATED_DIR="$DOCS_DIR/generated"

# Create generated docs directory
mkdir -p "$GENERATED_DIR"

echo "Generating documentation..."

# Generate CLI commands documentation
echo "  - CLI commands..."
HALVOR_CMD="halvor"
if ! command -v halvor &> /dev/null; then
    # Try to find halvor in common locations
    if [ -f "$REPO_ROOT/target/release/halvor" ]; then
        HALVOR_CMD="$REPO_ROOT/target/release/halvor"
    elif [ -f "$REPO_ROOT/target/debug/halvor" ]; then
        HALVOR_CMD="$REPO_ROOT/target/debug/halvor"
    fi
fi

if [ -f "$HALVOR_CMD" ] || command -v halvor &> /dev/null; then
    # Use halvor if available
    "$HALVOR_CMD" --help > "$GENERATED_DIR/cli-commands.txt" 2>&1 || true
    
    # Generate detailed help for each subcommand
    {
        echo "# CLI Commands Reference"
        echo ""
        echo "This document is auto-generated from the halvor CLI. Run \`halvor --help\` for the latest information."
        echo ""
        echo "Last updated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
        echo ""
        echo "## Main Commands"
        echo ""
        echo '```'
        "$HALVOR_CMD" --help 2>&1 || echo "halvor not found"
        echo '```'
        echo ""
        
        # Generate help for each major command
        for cmd in backup restore sync list install uninstall provision smb docker npm vpn config db update agent build dev generate k3s helm cluster; do
            echo "## \`halvor $cmd\`"
            echo ""
            echo '```'
            "$HALVOR_CMD" "$cmd" --help 2>&1 || echo "Command not available"
            echo '```'
            echo ""
        done
        
        # Generate help for subcommands
        echo "## Build Subcommands"
        echo ""
        echo '```'
        "$HALVOR_CMD" build --help 2>&1 || echo "Command not available"
        echo '```'
        echo ""
        
        echo "## K3s Subcommands"
        echo ""
        echo '```'
        "$HALVOR_CMD" k3s --help 2>&1 || echo "Command not available"
        echo '```'
        echo ""
        
        echo "## Helm Subcommands"
        echo ""
        echo '```'
        "$HALVOR_CMD" helm --help 2>&1 || echo "Command not available"
        echo '```'
        echo ""
        
        echo "## Agent Subcommands"
        echo ""
        echo '```'
        "$HALVOR_CMD" agent --help 2>&1 || echo "Command not available"
        echo '```'
        echo ""
        
    } > "$GENERATED_DIR/cli-commands.md"
else
    echo "    ⚠ halvor not found, skipping CLI docs generation"
    {
        echo "# CLI Commands Reference"
        echo ""
        echo "Run \`halvor --help\` after installation to see available commands."
        echo ""
        echo "To generate this documentation locally:"
        echo '```bash'
        echo "make docs"
        echo "# or"
        echo "./scripts/generate-docs.sh"
        echo '```'
    } > "$GENERATED_DIR/cli-commands.md"
fi

# Generate Docker containers documentation
echo "  - Docker containers..."
{
    echo "# Docker Containers"
    echo ""
    echo "This document lists all Docker containers that can be built and managed with halvor."
    echo ""
    echo "Last updated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo ""
    echo "## Available Containers"
    echo ""
    echo "### PIA VPN"
    echo ""
    echo "**Image**: \`ghcr.io/scottdkey/pia-vpn\`"
    echo ""
    echo "**Build Directory**: \`openvpn-container/\`"
    echo ""
    echo "**Description**: Private Internet Access VPN container with OpenVPN and Privoxy HTTP proxy."
    echo ""
    echo "**Build Command**:"
    echo '```bash'
    echo "halvor build pia-vpn [--no-cache] [--push] [--release]"
    echo '```'
    echo ""
    echo "**Usage**:"
    echo '```bash'
    echo "# Pull the image"
    echo "docker pull ghcr.io/scottdkey/pia-vpn:latest"
    echo ""
    echo "# Run the container"
    echo "docker run -d \\"
    echo "  --name pia-vpn \\"
    echo "  --cap-add=NET_ADMIN \\"
    echo "  --cap-add=NET_RAW \\"
    echo "  --device=/dev/net/tun \\"
    echo "  -p 8888:8888 \\"
    echo "  -e PIA_USERNAME=your_username \\"
    echo "  -e PIA_PASSWORD=your_password \\"
    echo "  -e REGION=us-california \\"
    echo "  -e UPDATE_CONFIGS=true \\"
    echo "  -v \$(pwd)/config/vpn:/config \\"
    echo "  ghcr.io/scottdkey/pia-vpn:latest"
    echo '```'
    echo ""
    echo "**Environment Variables**:"
    echo ""
    echo "- \`PIA_USERNAME\`: Your PIA username (required)"
    echo "- \`PIA_PASSWORD\`: Your PIA password (required)"
    echo "- \`REGION\`: VPN region (e.g., \`us-california\`, \`uk-london\`)"
    echo "- \`UPDATE_CONFIGS\`: Set to \`true\` to automatically download PIA configs"
    echo "- \`PROXY_PORT\`: Privoxy proxy port (default: \`8888\`)"
    echo "- \`DEBUG\`: Set to \`true\` for verbose logging"
    echo "- \`TZ\`: Timezone (default: \`Etc/UTC\`)"
    echo ""
    echo "**Features**:"
    echo ""
    echo "- Rust-based entrypoint for reliable process management"
    echo "- Automatic config download and updates"
    echo "- OpenVPN 2.6 compatibility fixes"
    echo "- Log tailing for both OpenVPN and Privoxy"
    echo "- Health checks and connectivity tests"
    echo ""
    echo "**Documentation**: See [VPN Setup Guide](vpn-setup.md) for detailed setup instructions."
    echo ""
} > "$GENERATED_DIR/docker-containers.md"

# Generate Helm charts documentation
echo "  - Helm charts..."
{
    echo "# Helm Charts"
    echo ""
    echo "This document lists all Helm charts available in this repository."
    echo ""
    echo "Last updated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
    echo ""
    echo "## Available Charts"
    echo ""
    
    # Find all Chart.yaml files
    for chart_yaml in "$REPO_ROOT/charts"/*/Chart.yaml; do
        if [ -f "$chart_yaml" ]; then
            chart_dir=$(dirname "$chart_yaml")
            chart_name=$(basename "$chart_dir")
            
            echo "### $chart_name"
            echo ""
            
            # Extract chart info from Chart.yaml
            if [ -f "$chart_yaml" ]; then
                chart_version=$(grep "^version:" "$chart_yaml" | sed 's/version: *//' | tr -d '"' || echo "N/A")
                chart_description=$(grep "^description:" "$chart_yaml" | sed 's/description: *//' | tr -d '"' || echo "No description")
                
                echo "**Version**: \`$chart_version\`"
                echo ""
                echo "**Description**: $chart_description"
                echo ""
            fi
            
            echo "**Chart Path**: \`charts/$chart_name/\`"
            echo ""
            
            # Check if values.yaml exists
            if [ -f "$chart_dir/values.yaml" ]; then
                echo "**Configuration**:"
                echo ""
                echo "Edit \`charts/$chart_name/values.yaml\` to customize deployment."
                echo ""
            fi
            
            echo "**Installation**:"
            echo '```bash'
            echo "# Install from local chart"
            echo "halvor helm install $chart_name"
            echo ""
            echo "# Or use helm directly"
            echo "helm install $chart_name ./charts/$chart_name"
            echo '```'
            echo ""
            
            # List templates if they exist
            if [ -d "$chart_dir/templates" ] && [ "$(ls -A $chart_dir/templates)" ]; then
                echo "**Resources**:"
                echo ""
                for template in "$chart_dir/templates"/*.yaml "$chart_dir/templates"/*.tpl; do
                    if [ -f "$template" ]; then
                        template_name=$(basename "$template")
                        echo "- \`$template_name\`"
                    fi
                done
                echo ""
            fi
            
            echo "---"
            echo ""
        fi
    done
    
    echo "## Using Helm Charts"
    echo ""
    echo "### Install a Chart"
    echo '```bash'
    echo "halvor helm install <chart-name>"
    echo '```'
    echo ""
    echo "### List Installed Charts"
    echo '```bash'
    echo "halvor helm list"
    echo '```'
    echo ""
    echo "### Uninstall a Chart"
    echo '```bash'
    echo "halvor helm uninstall <chart-name>"
    echo '```'
    echo ""
    echo "### Upgrade a Chart"
    echo '```bash'
    echo "halvor helm upgrade <chart-name>"
    echo '```'
    echo ""
    
} > "$GENERATED_DIR/helm-charts.md"

echo "✓ Documentation generated in $GENERATED_DIR/"
echo ""
echo "Generated files:"
echo "  - cli-commands.md"
echo "  - docker-containers.md"
echo "  - helm-charts.md"

