#!/bin/bash
# Temporary script to test OpenVPN connection to PIA

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="/tmp/openvpn-test.pid"
LOG_FILE="/tmp/openvpn-test.log"

echo "=== Testing OpenVPN Connection to PIA ==="
echo ""
echo "Config: $SCRIPT_DIR/openvpn/ca-montreal.ovpn"
echo "Auth: $SCRIPT_DIR/openvpn/auth.txt"
echo ""

# Detect local public IP before VPN connection
echo "Detecting your local public IP (before VPN)..."
LOCAL_PUBLIC_IP=$(curl -s --max-time 10 https://api.ipify.org 2>/dev/null || curl -s --max-time 10 https://ifconfig.me 2>/dev/null || curl -s --max-time 10 https://icanhazip.com 2>/dev/null || echo "unknown")

if [ "$LOCAL_PUBLIC_IP" = "unknown" ]; then
    echo "⚠ Warning: Could not detect local public IP"
    echo "  Continuing anyway..."
else
    echo "✓ Local public IP detected: $LOCAL_PUBLIC_IP"
fi
echo ""

# Start OpenVPN in background
echo "Starting OpenVPN connection..."
sudo /opt/homebrew/opt/openvpn/sbin/openvpn \
    --config "$SCRIPT_DIR/openvpn/ca-montreal.ovpn" \
    --auth-user-pass "$SCRIPT_DIR/openvpn/auth.txt" \
    --verb 3 \
    --log "$LOG_FILE" \
    --daemon \
    --writepid "$PID_FILE"

if [ ! -f "$PID_FILE" ]; then
    echo "✗ Failed to start OpenVPN"
    exit 1
fi

PID=$(cat "$PID_FILE")
echo "✓ OpenVPN started (PID: $PID)"
echo ""
echo "Waiting for connection to establish..."
sleep 8

# Check if process is still running
if ! ps -p $PID > /dev/null 2>&1; then
    echo "✗ OpenVPN process died"
    echo ""
    echo "Last 30 lines of log:"
    sudo tail -30 "$LOG_FILE" 2>/dev/null || echo "No log file"
    rm -f "$PID_FILE" "$LOG_FILE"
    exit 1
fi

# Check for TUN interface
if ifconfig | grep -q "tun0\|utun"; then
    echo "✓ TUN interface created"
    ifconfig | grep -A 5 "tun0\|utun" | head -10
else
    echo "⚠ TUN interface not found yet"
fi

echo ""
echo "Testing IP address after VPN connection..."
VPN_PUBLIC_IP=$(curl -s --max-time 10 https://api.ipify.org 2>/dev/null || curl -s --max-time 10 https://ifconfig.me 2>/dev/null || curl -s --max-time 10 https://icanhazip.com 2>/dev/null || echo "unknown")

if [ "$VPN_PUBLIC_IP" = "unknown" ]; then
    echo "⚠ Could not determine IP address after VPN"
elif [ "$VPN_PUBLIC_IP" = "$LOCAL_PUBLIC_IP" ] && [ "$LOCAL_PUBLIC_IP" != "unknown" ]; then
    echo "⚠ Warning: IP address unchanged ($VPN_PUBLIC_IP)"
    echo "  VPN may not be routing traffic correctly"
else
    echo "✓ Current IP: $VPN_PUBLIC_IP"
    if [ "$LOCAL_PUBLIC_IP" != "unknown" ]; then
        echo "  Changed from: $LOCAL_PUBLIC_IP"
        echo "  ✓ VPN is working - IP address changed!"
    else
        echo "  (This should be a PIA IP if connection is working)"
    fi
fi

echo ""
echo "Connection status:"
sudo tail -20 "$LOG_FILE" 2>/dev/null | grep -E "(Initialization Sequence|TLS|ERROR|WARNING)" || sudo tail -10 "$LOG_FILE" 2>/dev/null || echo "Could not read log file"

echo ""
echo "=== Test Complete ==="
echo ""
echo "Summary:"
echo "  Local public IP (before VPN): $LOCAL_PUBLIC_IP"
echo "  Public IP (after VPN): $VPN_PUBLIC_IP"
echo ""
echo "To stop this connection, run:"
echo "  sudo kill $PID"
echo "  rm -f $PID_FILE $LOG_FILE"
echo ""
echo "Or run the cleanup script:"
echo "  $SCRIPT_DIR/cleanup-openvpn-test.sh"
