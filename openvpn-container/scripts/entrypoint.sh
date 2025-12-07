#!/bin/bash

set -e

# PIA OpenVPN config download URL
PIA_CONFIG_URL="https://www.privateinternetaccess.com/openvpn/openvpn.zip"

# Download PIA configs if UPDATE_CONFIGS environment variable is set
if [ "${UPDATE_CONFIGS:-false}" = "true" ]; then
    echo "UPDATE_CONFIGS is set - downloading PIA OpenVPN configs..."
    
    # Ensure /config directory exists and is writable
    mkdir -p /config
    chmod 755 /config
    
    # Check if config directory is writable
    if [ ! -w /config ]; then
        echo "⚠ Warning: /config is not writable, cannot download configs"
        echo "  Make sure the volume mount is not read-only (:ro)"
    else
        # Create temp directory for download
        TEMP_DIR=$(mktemp -d)
        cd "$TEMP_DIR"
        
        echo "Downloading PIA OpenVPN configs from: $PIA_CONFIG_URL"
        if wget -q "$PIA_CONFIG_URL" -O openvpn.zip; then
            echo "✓ Download successful"
            
            # Extract configs
            if unzip -q openvpn.zip; then
                echo "✓ Extraction successful"
                
                # Copy .ovpn files to /config
                find . -name "*.ovpn" -exec cp {} /config/ \;
                
                echo "✓ Configs copied to /config"
            else
                echo "⚠ Failed to extract configs"
            fi
            
            # Cleanup
            cd /
            rm -rf "$TEMP_DIR"
        else
            echo "⚠ Failed to download configs - continuing with existing configs"
        fi
    fi
fi

# Handle PIA credentials from environment variables
if [ -n "${PIA_USERNAME:-}" ] && [ -n "${PIA_PASSWORD:-}" ]; then
    echo "PIA_USERNAME and PIA_PASSWORD provided - creating/updating auth.txt..."
    if [ -w /config ]; then
        echo "$PIA_USERNAME" > /config/auth.txt
        echo "$PIA_PASSWORD" >> /config/auth.txt
        chmod 600 /config/auth.txt
        echo "✓ auth.txt created/updated from environment variables"
    else
        echo "⚠ Warning: Cannot write to /config (volume may be read-only)"
        echo "  Please ensure volume mount is writable or provide auth.txt manually"
        echo "  Remove :ro from volume mount if using UPDATE_CONFIGS or PIA_USERNAME/PIA_PASSWORD"
    fi
fi

# Find OpenVPN config file based on REGION or default
OVPN_CONFIG=""
REGION="${REGION:-}"

echo "Checking for OpenVPN config files in /config..."
echo "Contents of /config:"
ls -la /config/ 2>&1 || echo "Cannot list /config directory"

# If REGION is specified, try to find matching config
if [ -n "$REGION" ]; then
    echo "REGION specified: $REGION"
    # PIA configs are named like: us-california.ovpn, ca-montreal.ovpn, etc.
    # Try to find a config that matches the region (case-insensitive)
    REGION_LOWER=$(echo "$REGION" | tr '[:upper:]' '[:lower:]')
    
    # Try exact match first
    if [ -f "/config/${REGION_LOWER}.ovpn" ]; then
        OVPN_CONFIG="/config/${REGION_LOWER}.ovpn"
        echo "Found exact match: $OVPN_CONFIG"
    else
        # Try to find configs that contain the region name
        MATCHING_CONFIG=$(find /config -name "*.ovpn" -type f | grep -i "$REGION_LOWER" | head -1)
        if [ -n "$MATCHING_CONFIG" ]; then
            OVPN_CONFIG="$MATCHING_CONFIG"
            echo "Found region match: $OVPN_CONFIG"
        else
            echo "⚠ No config found matching region: $REGION"
            echo "Available configs:"
            ls -1 /config/*.ovpn 2>/dev/null | sed 's|^/config/||' | sed 's|^|  - |' || echo "  (none found)"
        fi
    fi
fi

# If no config found yet, try default fallback logic
if [ -z "$OVPN_CONFIG" ]; then
    # Try common default: ca-montreal.ovpn
    if [ -f /config/ca-montreal.ovpn ]; then
        OVPN_CONFIG="/config/ca-montreal.ovpn"
        echo "Using default config: $OVPN_CONFIG"
    # Otherwise, use first available .ovpn file
    elif ls /config/*.ovpn 1> /dev/null 2>&1; then
        OVPN_CONFIG=$(ls /config/*.ovpn | head -1)
        echo "Using first available config: $OVPN_CONFIG"
    else
        echo "⚠ No OpenVPN config file found in /config"
        echo ""
        echo "Debugging information:"
        echo "  /config exists: $([ -d /config ] && echo 'yes' || echo 'no')"
        echo "  /config readable: $([ -r /config ] && echo 'yes' || echo 'no')"
        echo "  /config writable: $([ -w /config ] && echo 'yes' || echo 'no')"
        echo "  Files in /config:"
        find /config -type f 2>&1 | head -10 || echo "  Cannot search /config"
        echo ""
        echo "Please ensure:"
        echo "  1. Directory \$HOME/config/vpn exists on the host"
        echo "  2. Files are present: \$HOME/config/vpn/<region>.ovpn and \$HOME/config/vpn/auth.txt"
        echo "  3. Docker daemon has access to \$HOME/config/vpn (check volume mount path)"
        echo "  4. Or set UPDATE_CONFIGS=true to download configs automatically"
        echo "  5. Set REGION environment variable to select a specific region (e.g., REGION=us-california)"
        exit 1
    fi
fi

echo "Using OpenVPN config: $OVPN_CONFIG"

# Check for auth file
if [ ! -f /config/auth.txt ]; then
    echo "⚠ Warning: /config/auth.txt not found"
    echo "OpenVPN may fail without authentication credentials"
    echo ""
    echo "Options:"
    echo "  1. Set PIA_USERNAME and PIA_PASSWORD environment variables"
    echo "  2. Create /config/auth.txt manually with format:"
    echo "     Line 1: PIA username"
    echo "     Line 2: PIA password"
fi

# Start Privoxy in background first (so it's ready when OpenVPN connects)
echo "Starting Privoxy..."
privoxy --no-daemon /etc/privoxy/config &
PRIVOXY_PID=$!

# Function to cleanup on exit
cleanup() {
    echo "Shutting down..."
    if [ -n "$OPENVPN_PID" ] && kill -0 "$OPENVPN_PID" 2>/dev/null; then
        kill "$OPENVPN_PID" 2>/dev/null || true
    fi
    if [ -n "$PRIVOXY_PID" ] && kill -0 "$PRIVOXY_PID" 2>/dev/null; then
        kill "$PRIVOXY_PID" 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGTERM SIGINT

# Start OpenVPN
echo "Starting OpenVPN..."
openvpn \
    --config "$OVPN_CONFIG" \
    --auth-user-pass /config/auth.txt \
    --daemon \
    --log /var/log/openvpn/openvpn.log \
    --mssfix 1450 \
    --fragment 1450 \
    --sndbuf 393216 \
    --rcvbuf 393216 \
    --verb 3

# Wait for OpenVPN to start and connect
echo "Waiting for OpenVPN connection..."
sleep 8

# Check if OpenVPN is running
OPENVPN_PID=$(pgrep -f "openvpn.*$OVPN_CONFIG" || echo "")
if [ -z "$OPENVPN_PID" ]; then
    echo "⚠ OpenVPN process not found, checking logs..."
    tail -20 /var/log/openvpn/openvpn.log || true
    exit 1
fi

# Check if TUN interface is up
if ! ip link show tun0 >/dev/null 2>&1; then
    echo "⚠ TUN interface (tun0) not found"
    echo "OpenVPN may not have connected successfully"
    tail -30 /var/log/openvpn/openvpn.log || true
fi

# Check for "Initialization Sequence Completed" in logs
if ! grep -q "Initialization Sequence Completed" /var/log/openvpn/openvpn.log 2>/dev/null; then
    echo "⚠ OpenVPN may not have completed initialization"
    echo "Recent logs:"
    tail -20 /var/log/openvpn/openvpn.log || true
fi

echo "✓ OpenVPN started (PID: $OPENVPN_PID)"
echo "✓ Privoxy started (PID: $PRIVOXY_PID)"
echo ""
echo "VPN Status:"
ip addr show tun0 2>/dev/null || echo "  TUN interface: Not available"
echo "  Privoxy proxy: http://0.0.0.0:8888"
echo ""

# Wait for processes
wait $OPENVPN_PID $PRIVOXY_PID
