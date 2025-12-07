#!/bin/bash

# Script to tail both OpenVPN and Privoxy logs simultaneously
# Usage: 
#   docker exec <container> tail-logs.sh
#   Or run inside container: /usr/local/bin/tail-logs.sh

set -e

OPENVPN_LOG="/var/log/openvpn/openvpn.log"
PRIVOXY_LOG="/var/log/privoxy/logfile"

# Check if log files exist
if [ ! -f "$OPENVPN_LOG" ]; then
    echo "⚠ Warning: OpenVPN log file not found: $OPENVPN_LOG"
    echo "  OpenVPN may not have started yet"
fi

if [ ! -f "$PRIVOXY_LOG" ]; then
    echo "⚠ Warning: Privoxy log file not found: $PRIVOXY_LOG"
    echo "  Privoxy may not have started yet"
fi

# Function to cleanup on exit
cleanup() {
    echo ""
    echo "Stopping log tail..."
    kill $TAIL_PIDS 2>/dev/null || true
    exit 0
}

trap cleanup SIGTERM SIGINT

echo "=== Tailing OpenVPN and Privoxy Logs ==="
echo "Press Ctrl+C to stop"
echo ""

# Tail both logs with prefixes
if [ -f "$OPENVPN_LOG" ] && [ -f "$PRIVOXY_LOG" ]; then
    # Both logs exist - tail both with prefixes
    tail -f "$OPENVPN_LOG" | sed 's/^/[OpenVPN] /' &
    TAIL_PIDS="$! "
    tail -f "$PRIVOXY_LOG" | sed 's/^/[Privoxy] /' &
    TAIL_PIDS="${TAIL_PIDS}$!"
elif [ -f "$OPENVPN_LOG" ]; then
    # Only OpenVPN log exists
    tail -f "$OPENVPN_LOG" | sed 's/^/[OpenVPN] /' &
    TAIL_PIDS="$!"
elif [ -f "$PRIVOXY_LOG" ]; then
    # Only Privoxy log exists
    tail -f "$PRIVOXY_LOG" | sed 's/^/[Privoxy] /' &
    TAIL_PIDS="$!"
else
    echo "⚠ No log files found. Waiting for services to start..."
    # Wait a bit and try again
    sleep 5
    if [ -f "$OPENVPN_LOG" ] || [ -f "$PRIVOXY_LOG" ]; then
        exec "$0"
    else
        echo "⚠ Log files still not found after waiting"
        exit 1
    fi
fi

# Wait for tail processes
wait $TAIL_PIDS
