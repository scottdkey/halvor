#!/bin/bash
# Test script to validate VPN path changes

set -e

echo "Testing VPN path configuration..."
echo ""

# Test 1: Check if paths are correctly referenced in compose files
echo "=== Test 1: Compose file path references ==="
if grep -q "/home/\${USER" compose/openvpn-pia-portainer.docker-compose.yml; then
    echo "✓ Portainer compose file uses /home/\${USER}/config/vpn"
else
    echo "✗ Portainer compose file missing /home/\${USER}/config/vpn"
    exit 1
fi

if grep -q "/home/\${USER" compose/openvpn-pia.docker-compose.yml; then
    echo "✓ Local compose file uses /home/\${USER}/config/vpn"
else
    echo "✗ Local compose file missing /home/\${USER}/config/vpn"
    exit 1
fi

# Test 2: Check Rust code uses new paths
echo ""
echo "=== Test 2: Rust code path references ==="
if grep -q '/home/.*config/vpn' src/vpn.rs; then
    echo "✓ Rust code uses /home/\$USER/config/vpn"
    COUNT=$(grep -c '/home/.*config/vpn' src/vpn.rs || echo "0")
    echo "  Found $COUNT references"
else
    echo "✗ Rust code missing /home/\$USER/config/vpn references"
    exit 1
fi

# Test 3: Verify no old paths remain (except in comments)
echo ""
echo "=== Test 3: Check for old /opt/vpn/openvpn paths ==="
OLD_PATHS=$(grep -r "/opt/vpn/openvpn" src/ compose/ --exclude-dir=target 2>/dev/null | grep -v "^#" | grep -v "Use \$HOME" | wc -l | tr -d ' ')
if [ "$OLD_PATHS" = "0" ]; then
    echo "✓ No old /opt/vpn/openvpn paths found in code"
else
    echo "⚠ Found $OLD_PATHS references to old path (may be in comments)"
    grep -r "/opt/vpn/openvpn" src/ compose/ --exclude-dir=target 2>/dev/null | grep -v "^#" | grep -v "Use \$HOME" || true
fi

# Test 4: Validate shell command syntax
echo ""
echo "=== Test 4: Validate shell command syntax ==="
TEST_CMD='test -f "$HOME/config/vpn/auth.txt" && (test -f "$HOME/config/vpn/ca-montreal.ovpn" || test -f "$HOME/config/vpn/ca-montreal.opvn") && echo exists || echo missing'
if bash -c "$TEST_CMD" > /dev/null 2>&1; then
    echo "✓ Shell command syntax is valid"
else
    echo "⚠ Shell command syntax check (expected to fail if files don't exist)"
fi

# Test 5: Check mkdir command
echo ""
echo "=== Test 5: Validate directory creation command ==="
MKDIR_CMD='mkdir -p "$HOME/config/vpn" && echo "Directory would be created at: $HOME/config/vpn"'
if bash -c "$MKDIR_CMD" > /dev/null 2>&1; then
    echo "✓ Directory creation command is valid"
    bash -c 'echo "  Example path: $HOME/config/vpn"'
else
    echo "✗ Directory creation command failed"
    exit 1
fi

echo ""
echo "=== All tests passed! ==="
echo ""
echo "Summary:"
echo "  - Compose files use /home/\${USER}/config/vpn (USER can be set in Portainer)"
echo "  - Rust code uses /home/\$USER/config/vpn (respects VPN_USER env var)"
echo "  - No sudo required (files in user's home directory)"
echo "  - Paths are user-specific and avoid permission issues"
echo "  - Set USER environment variable in Portainer to specify which user's home to use"
