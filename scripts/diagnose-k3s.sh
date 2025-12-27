#!/bin/bash
# Diagnose K3s join issues on a remote host via halvor agent

HOSTNAME="${1:-oak}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Diagnosing K3s join issues on: $HOSTNAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Discover agents
echo "1. Discovering halvor agents..."
DISCOVERED=$(halvor agent discover 2>&1)
echo "$DISCOVERED"
echo ""

# Check if oak is in the discovered agents
if echo "$DISCOVERED" | grep -qi "$HOSTNAME"; then
    echo "✓ Found $HOSTNAME in discovered agents"
else
    echo "⚠ $HOSTNAME not found in discovered agents"
    echo "  Trying to connect anyway..."
fi
echo ""

# Use halvor agent to execute diagnostic commands
echo "2. Checking K3s installation..."
halvor agent execute "$HOSTNAME" "test -f /usr/local/bin/k3s && echo 'K3s binary exists' || echo 'K3s binary NOT found'" 2>&1 || echo "Failed to check K3s binary"
echo ""

echo "3. Checking K3s service status..."
halvor agent execute "$HOSTNAME" "systemctl is-active k3s 2>/dev/null || systemctl is-active k3s-agent 2>/dev/null || echo 'K3s service not running'" 2>&1 || echo "Failed to check service status"
echo ""

echo "4. Checking K3s service logs (last 20 lines)..."
halvor agent execute "$HOSTNAME" "journalctl -u k3s -n 20 --no-pager 2>/dev/null || journalctl -u k3s-agent -n 20 --no-pager 2>&1 || echo 'No K3s logs found'" 2>&1 || echo "Failed to get logs"
echo ""

echo "5. Checking network connectivity..."
halvor agent execute "$HOSTNAME" "ping -c 1 -W 2 frigg.bombay-pinecone.ts.net 2>&1 || echo 'Cannot ping frigg'" 2>&1 || echo "Failed to check connectivity"
echo ""

echo "6. Checking Tailscale status..."
halvor agent execute "$HOSTNAME" "tailscale status --json 2>&1 | head -5 || tailscale status 2>&1 | head -5 || echo 'Tailscale not running'" 2>&1 || echo "Failed to check Tailscale"
echo ""

echo "7. Checking system architecture..."
halvor agent execute "$HOSTNAME" "uname -m" 2>&1 || echo "Failed to check architecture"
echo ""

echo "8. Checking OS..."
halvor agent execute "$HOSTNAME" "uname -s" 2>&1 || echo "Failed to check OS"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Diagnosis complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

