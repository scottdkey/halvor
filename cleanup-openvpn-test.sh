#!/bin/bash
# Cleanup script for OpenVPN test connection

PID_FILE="/tmp/openvpn-test.pid"
LOG_FILE="/tmp/openvpn-test.log"

echo "=== Cleaning up OpenVPN test connection ==="
echo ""

if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    echo "Stopping OpenVPN (PID: $PID)..."
    sudo kill $PID 2>/dev/null || true
    sleep 2
    sudo kill -9 $PID 2>/dev/null || true
    rm -f "$PID_FILE"
    echo "✓ OpenVPN stopped"
else
    echo "No PID file found"
fi

# Clean up any remaining OpenVPN processes with our config
REMAINING=$(pgrep -f "ca-montreal.ovpn" || true)
if [ -n "$REMAINING" ]; then
    echo "Found remaining OpenVPN processes: $REMAINING"
    echo "Killing them..."
    sudo kill $REMAINING 2>/dev/null || true
    sleep 1
    sudo kill -9 $REMAINING 2>/dev/null || true
fi

# Remove log file
if [ -f "$LOG_FILE" ]; then
    rm -f "$LOG_FILE"
    echo "✓ Log file removed"
fi

echo ""
echo "✓ Cleanup complete"
