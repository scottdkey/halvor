#!/bin/bash
set -e

echo "=== OpenVPN PIA Container ==="
echo ""

# Download PIA configs if UPDATE_CONFIGS environment variable is set
if [ "${UPDATE_CONFIGS:-}" = "true" ] || [ "${UPDATE_CONFIGS:-}" = "1" ]; then
    echo "UPDATE_CONFIGS is set - downloading PIA OpenVPN configs..."
    echo ""
    
    CONFIG_DIR="/config"
    PIA_CONFIG_URL="https://www.privateinternetaccess.com/openvpn/openvpn.zip"
    
    # Create temp directory for download
    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"
    
    echo "Downloading PIA OpenVPN configs from: $PIA_CONFIG_URL"
    if wget -q "$PIA_CONFIG_URL" -O openvpn.zip; then
        echo "✓ Download successful"
        
        # Extract zip file
        if unzip -q openvpn.zip; then
            echo "✓ Extraction successful"
            
            # Copy all .ovpn files to /config
            if [ -d "openvpn" ]; then
                cp openvpn/*.ovpn "$CONFIG_DIR/" 2>/dev/null || true
                echo "✓ Copied config files to $CONFIG_DIR"
                echo ""
                echo "Available config files:"
                ls -1 "$CONFIG_DIR"/*.ovpn 2>/dev/null | xargs -n1 basename || echo "  (none found)"
            else
                # Try root of zip
                cp *.ovpn "$CONFIG_DIR/" 2>/dev/null || true
                echo "✓ Copied config files to $CONFIG_DIR"
            fi
            
            # Cleanup
            cd /
            rm -rf "$TEMP_DIR"
            echo ""
            echo "✓ PIA configs updated"
        else
            echo "⚠ Failed to extract zip file"
            rm -rf "$TEMP_DIR"
        fi
    else
        echo "⚠ Failed to download configs - continuing with existing configs"
        rm -rf "$TEMP_DIR"
    fi
    echo ""
fi

# Check if config file exists (try ca-montreal.ovpn first, then any .ovpn file)
CONFIG_FILE=""
if [ -f /config/ca-montreal.ovpn ]; then
    CONFIG_FILE="/config/ca-montreal.ovpn"
elif [ -f /config/*.ovpn ]; then
    CONFIG_FILE=$(ls -1 /config/*.ovpn | head -1)
    echo "Using config file: $CONFIG_FILE"
fi

if [ -z "$CONFIG_FILE" ] || [ ! -f "$CONFIG_FILE" ]; then
    echo "ERROR: No OpenVPN config file found in /config/"
    echo "Please mount your OpenVPN config file to /config/ or set UPDATE_CONFIGS=true"
    exit 1
fi

# Check if auth file exists
if [ ! -f /config/auth.txt ]; then
    echo "ERROR: /config/auth.txt not found!"
    echo "Please mount your auth file to /config/auth.txt"
    exit 1
fi

echo "Starting OpenVPN..."
echo "  Config: $CONFIG_FILE"
echo "  Auth: /config/auth.txt"
echo ""

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Shutting down..."
    kill $PRIVOXY_PID 2>/dev/null || true
    kill $OPENVPN_PID 2>/dev/null || true
    exit 0
}

trap cleanup SIGTERM SIGINT

# Start OpenVPN using daemon mode
# Add Docker-specific options to handle network issues:
# - mssfix: Handle MTU issues in Docker networks
# - fragment: Handle packet fragmentation
# - sndbuf/rcvbuf: Optimize buffer sizes for container networking
openvpn \
    --config "$CONFIG_FILE" \
    --auth-user-pass /config/auth.txt \
    --log /var/log/openvpn/openvpn.log \
    --verb 3 \
    --writepid /var/run/openvpn.pid \
    --mssfix 1450 \
    --fragment 1450 \
    --sndbuf 393216 \
    --rcvbuf 393216 \
    --daemon

# Wait for daemon to start and create PID file
sleep 2

# Get PID from file
if [ -f /var/run/openvpn.pid ]; then
    OPENVPN_PID=$(cat /var/run/openvpn.pid)
    echo "✓ OpenVPN started (PID: $OPENVPN_PID)"
else
    echo "⚠ Warning: PID file not found, checking process..."
    OPENVPN_PID=$(pgrep -f "openvpn.*$CONFIG_FILE" | head -1 || echo "")
    if [ -z "$OPENVPN_PID" ]; then
        echo "ERROR: OpenVPN failed to start"
        echo "Checking logs..."
        tail -30 /var/log/openvpn/openvpn.log 2>/dev/null || echo "No log file found"
        exit 1
    fi
    echo "✓ OpenVPN found (PID: $OPENVPN_PID)"
fi

# Wait for connection to establish (same as host test - 8 seconds)
echo ""
echo "Waiting for connection to establish..."
sleep 8

# Check if process is still running
if ! kill -0 $OPENVPN_PID 2>/dev/null; then
    echo "✗ OpenVPN process died"
    echo ""
    echo "Last 30 lines of log:"
    tail -30 /var/log/openvpn/openvpn.log 2>/dev/null || echo "No log file found"
    exit 1
fi

# Check for TUN interface (VPN connection established)
TUN_INTERFACE=$(ip link show | grep -o 'tun[0-9]*' | head -1 || echo "")
if [ -n "$TUN_INTERFACE" ]; then
    echo "✓ TUN interface created: $TUN_INTERFACE"
else
    echo "⚠ Warning: TUN interface not found yet"
    echo "  Checking logs for connection status..."
    tail -20 /var/log/openvpn/openvpn.log 2>/dev/null | grep -E "(Initialization Sequence|TLS|ERROR)" || tail -10 /var/log/openvpn/openvpn.log 2>/dev/null
fi

# Verify connection is established by checking logs
if grep -q "Initialization Sequence Completed" /var/log/openvpn/openvpn.log 2>/dev/null; then
    echo "✓ VPN connection established successfully"
else
    echo "⚠ Warning: Connection may not be fully established"
    echo "  Checking recent log entries..."
    tail -10 /var/log/openvpn/openvpn.log 2>/dev/null
fi

echo ""
echo "Starting Privoxy HTTP proxy..."
echo "  Listening on: 0.0.0.0:8888"
echo "  Traffic will route through VPN TUN interface"
privoxy --no-daemon /etc/privoxy/config &
PRIVOXY_PID=$!

# Wait a moment for Privoxy to start
sleep 2

# Check if Privoxy is still running
if ! kill -0 $PRIVOXY_PID 2>/dev/null; then
    echo "ERROR: Privoxy failed to start"
    kill $OPENVPN_PID 2>/dev/null || true
    exit 1
fi

echo "✓ Privoxy started (PID: $PRIVOXY_PID)"
echo ""
echo "=== Container Ready ==="
echo "  OpenVPN: Running (PID: $OPENVPN_PID)"
echo "  Privoxy: Running (PID: $PRIVOXY_PID) on port 8888"
echo "  VPN TUN: $TUN_INTERFACE"
echo ""

# Monitor both processes
while kill -0 $OPENVPN_PID 2>/dev/null && kill -0 $PRIVOXY_PID 2>/dev/null; do
    sleep 5
done

EXIT_CODE=$?
echo ""
echo "One of the processes exited (OpenVPN: $OPENVPN_PID, Privoxy: $PRIVOXY_PID)"

# Cleanup
kill $PRIVOXY_PID 2>/dev/null || true
kill $OPENVPN_PID 2>/dev/null || true

exit $EXIT_CODE
