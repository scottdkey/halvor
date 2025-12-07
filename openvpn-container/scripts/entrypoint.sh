#!/bin/bash

set -e

# Debug: Print environment variables if DEBUG is set
if [ "${DEBUG:-false}" = "true" ]; then
    echo "=== Environment Variables ==="
    echo "REGION: ${REGION:-<not set>}"
    echo "PIA_USERNAME: ${PIA_USERNAME:-<not set>}"
    echo "PIA_PASSWORD: ${PIA_PASSWORD:+<set>}"
    echo "UPDATE_CONFIGS: ${UPDATE_CONFIGS:-<not set>}"
    echo "TZ: ${TZ:-<not set>}"
    echo "=============================="
    echo ""
fi

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
    # PIA configs are named like: us_california.ovpn, uk_london.ovpn, canada.ovpn, etc.
    # Normalize region name: convert to lowercase and replace hyphens with underscores
    REGION_NORMALIZED=$(echo "$REGION" | tr '[:upper:]' '[:lower:]' | tr '-' '_')
    
    echo "Normalized region: $REGION_NORMALIZED"
    
    # Try exact match first (with .ovpn extension)
    if [ -f "/config/${REGION_NORMALIZED}.ovpn" ]; then
        OVPN_CONFIG="/config/${REGION_NORMALIZED}.ovpn"
        echo "✓ Found exact match: $OVPN_CONFIG"
    else
        # Try partial match - find configs that contain the region name
        # This handles cases like "montreal" matching "canada" or "montreal" in filename
        MATCHING_CONFIG=$(find /config -name "*.ovpn" -type f | grep -i "$REGION_NORMALIZED" | head -1)
        
        if [ -n "$MATCHING_CONFIG" ]; then
            OVPN_CONFIG="$MATCHING_CONFIG"
            echo "✓ Found partial match: $OVPN_CONFIG"
        else
            # Try matching just the last part (e.g., "montreal" or "california")
            # Extract the last segment after underscore or hyphen
            REGION_PART=$(echo "$REGION_NORMALIZED" | awk -F'[_-]' '{print $NF}')
            if [ "$REGION_PART" != "$REGION_NORMALIZED" ]; then
                MATCHING_CONFIG=$(find /config -name "*.ovpn" -type f | grep -i "$REGION_PART" | head -1)
                if [ -n "$MATCHING_CONFIG" ]; then
                    OVPN_CONFIG="$MATCHING_CONFIG"
                    echo "✓ Found match by region part '$REGION_PART': $OVPN_CONFIG"
                fi
            fi
            
            if [ -z "$OVPN_CONFIG" ]; then
                echo "⚠ No config found matching region: $REGION"
                echo "  Tried: ${REGION_NORMALIZED}.ovpn"
                echo "  Tried: partial match for '$REGION_NORMALIZED'"
                if [ "$REGION_PART" != "$REGION_NORMALIZED" ]; then
                    echo "  Tried: partial match for '$REGION_PART'"
                fi
                echo ""
                echo "Available configs (first 20):"
                ls -1 /config/*.ovpn 2>/dev/null | sed 's|^/config/||' | sed 's|^|  - |' | head -20 || echo "  (none found)"
            fi
        fi
    fi
fi

# If no config found yet, try default fallback logic
if [ -z "$OVPN_CONFIG" ]; then
    # Use first available .ovpn file (alphabetically sorted for consistency)
    if ls /config/*.ovpn 1> /dev/null 2>&1; then
        OVPN_CONFIG=$(ls /config/*.ovpn | sort | head -1)
        echo "No REGION specified - using first available config: $OVPN_CONFIG"
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
    echo ""
    echo "Shutting down..."
    # Kill OpenVPN by finding the process
    OPENVPN_PID=$(pgrep -f "openvpn.*$OVPN_CONFIG" || echo "")
    if [ -n "$OPENVPN_PID" ]; then
        echo "Stopping OpenVPN (PID: $OPENVPN_PID)..."
        kill "$OPENVPN_PID" 2>/dev/null || true
        # Wait a bit for graceful shutdown
        sleep 2
        # Force kill if still running
        if kill -0 "$OPENVPN_PID" 2>/dev/null; then
            kill -9 "$OPENVPN_PID" 2>/dev/null || true
        fi
    fi
    # Kill Privoxy (child process)
    if [ -n "$PRIVOXY_PID" ] && kill -0 "$PRIVOXY_PID" 2>/dev/null; then
        echo "Stopping Privoxy (PID: $PRIVOXY_PID)..."
        kill "$PRIVOXY_PID" 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGTERM SIGINT

# Start OpenVPN
echo "Starting OpenVPN..."
# Note: OpenVPN config should include 'redirect-gateway def1' to route all traffic through VPN
# This ensures container traffic uses VPN IP, while host maintains public IP
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
echo ""
echo "Network Configuration:"
echo "  Container IPs:"
ip addr show | grep "inet " | grep -v "127.0.0.1" | sed 's/^/    /'
echo "  Default route:"
ip route | grep default | sed 's/^/    /'
echo "  Privoxy listening on:"
netstat -tlnp 2>/dev/null | grep 8888 || ss -tlnp 2>/dev/null | grep 8888 || echo "    (checking...)"
echo ""
echo "Proxy Information:"
echo "  Privoxy proxy: http://0.0.0.0:8888 (inside container)"
echo "  Access from host: http://<host-ip>:8888"
echo "  Access from other containers: http://openvpn-pia:8888"
echo ""
echo "Diagnostics:"
echo "  View logs: docker exec <container> tail-logs.sh"
echo "  Check proxy: docker exec <container> check-proxy.sh"
echo ""

# Wait for Privoxy (child process) and monitor OpenVPN
# OpenVPN runs as a daemon, so we can't wait on it directly
# Instead, we monitor it by checking if the process exists
while kill -0 "$PRIVOXY_PID" 2>/dev/null; do
    # Check if OpenVPN is still running
    if ! pgrep -f "openvpn.*$OVPN_CONFIG" >/dev/null 2>&1; then
        echo "⚠ OpenVPN process exited unexpectedly"
        tail -30 /var/log/openvpn/openvpn.log || true
        cleanup
        exit 1
    fi
    # Sleep briefly before checking again
    sleep 5
done

# If Privoxy exits, cleanup and exit
echo "⚠ Privoxy exited"
cleanup
