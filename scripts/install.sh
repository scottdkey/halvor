#!/bin/bash

# Cross-platform installation script for halvor
# Downloads and installs the pre-built binary from GitHub releases

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# GitHub repository (update if forked)
GITHUB_REPO="${GITHUB_REPO:-scottdkey/halvor}"
GITHUB_API="https://api.github.com/repos/${GITHUB_REPO}"
GITHUB_RAW="https://raw.githubusercontent.com/${GITHUB_REPO}"

echo "Installing halvor (Homelab Automation Layer)..."

# Detect OS and architecture
detect_platform() {
    local OS
    local ARCH
    local EXT=""
    
    case "$(uname -s)" in
        Linux*)
            OS="linux"
            # Check if musl (Alpine)
            if ldd --version 2>&1 | grep -q musl; then
                EXT="musl"
            fi
            ;;
        Darwin*)
            OS="darwin"
            ;;
        *)
            echo -e "${RED}Unsupported operating system: $(uname -s)${NC}"
            exit 1
            ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64)
            ARCH="amd64"
            ;;
        aarch64|arm64)
            ARCH="arm64"
            ;;
        *)
            echo -e "${RED}Unsupported architecture: $(uname -m)${NC}"
            exit 1
            ;;
    esac
    
    echo "${OS}-${ARCH}${EXT:+-${EXT}}"
}

# Get latest release version
get_latest_version() {
    local version
    version=$(curl -s "${GITHUB_API}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/' || echo "")
    
    if [ -z "$version" ]; then
        echo -e "${YELLOW}Warning: Could not fetch latest version, using 'latest'${NC}"
        echo "latest"
    else
        echo "$version"
    fi
}

# Download binary from GitHub releases
download_binary() {
    local version=$1
    local platform=$2
    local download_url
    
    if [ "$version" = "latest" ]; then
        # Try to get from latest release
        download_url=$(curl -s "${GITHUB_API}/releases/latest" | grep "browser_download_url.*halvor-.*-${platform}.tar.gz" | cut -d '"' -f 4 | head -1)
    else
        # Get from specific release
        download_url=$(curl -s "${GITHUB_API}/releases/tags/${version}" | grep "browser_download_url.*halvor-.*-${platform}.tar.gz" | cut -d '"' -f 4 | head -1)
    fi
    
    if [ -z "$download_url" ]; then
        echo -e "${RED}Error: Could not find download URL for platform ${platform}${NC}" >&2
        echo "Available releases: https://github.com/${GITHUB_REPO}/releases" >&2
        exit 1
    fi
    
    echo -e "${GREEN}Downloading from: ${download_url}${NC}" >&2
    
    local temp_dir=$(mktemp -d)
    local archive_path="${temp_dir}/halvor.tar.gz"
    
    curl -L -o "$archive_path" "$download_url" || {
        echo -e "${RED}Error: Failed to download binary${NC}" >&2
        rm -rf "$temp_dir"
        exit 1
    }
    
    tar -xzf "$archive_path" -C "$temp_dir"
    echo "$temp_dir/halvor"
}

PLATFORM=$(detect_platform)

# Parse arguments
VERSION="latest"
EXPERIMENTAL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --experimental|-e)
            EXPERIMENTAL=true
            VERSION="experimental"
            shift
            ;;
        --version|-v)
            VERSION="$2"
            shift 2
            ;;
        *)
            # Treat as version if not a flag
            if [ "$VERSION" = "latest" ] && [ "$1" != "--experimental" ] && [ "$1" != "-e" ]; then
                VERSION="$1"
            fi
            shift
            ;;
    esac
done

# If experimental flag was set, use it
if [ "$EXPERIMENTAL" = "true" ]; then
    VERSION="experimental"
fi

if [ "$VERSION" != "latest" ] && [ "$VERSION" != "experimental" ]; then
    VERSION="v${VERSION#v}"  # Ensure 'v' prefix
fi

if [ "$VERSION" = "latest" ]; then
    VERSION=$(get_latest_version)
fi

echo -e "${GREEN}Platform: ${PLATFORM}${NC}"
echo -e "${GREEN}Version: ${VERSION}${NC}"

# Determine installation directory
if [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -w "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
else
    echo -e "${RED}Error: Cannot write to /usr/local/bin or ~/.local/bin${NC}"
    echo "Please run with sudo or create ~/.local/bin directory"
    exit 1
fi

INSTALL_PATH="$INSTALL_DIR/halvor"

# Check if halvor already exists
if [ -e "$INSTALL_PATH" ]; then
    echo -e "${YELLOW}halvor already exists at $INSTALL_PATH${NC}"
    read -p "Overwrite? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    rm -f "$INSTALL_PATH"
fi

# Download and install
echo "Downloading halvor binary..."
BINARY_PATH=$(download_binary "$VERSION" "$PLATFORM")

# Move binary to install location
mv "$BINARY_PATH" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

# Cleanup
rm -rf "$(dirname "$BINARY_PATH")"

echo -e "${GREEN}✓ halvor installed to $INSTALL_PATH${NC}"

# Check if install directory is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH${NC}"
    echo "Add this to your ~/.bashrc, ~/.zshrc, or ~/.profile:"
    echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Configure halvor: halvor config init"
echo "     (This sets up the path to your .env file)"
echo "  2. Setup cluster: halvor k3s setup --primary <node> --nodes <node1>,<node2>"
echo "  3. See all commands: halvor --help"
