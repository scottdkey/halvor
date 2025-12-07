#!/bin/bash

# Script to test VPN connection and proxy functionality
# Usage: docker exec <container> test-vpn.sh

set +e  # Don't exit on errors, we want to see all test results

echo "=== VPN Connection Test ==="
echo ""

# Test 1: Check if OpenVPN is running
echo "1. Checking OpenVPN status..."
if pgrep -f "openvpn" >/dev/null; then
    echo "   ✓ OpenVPN is running"
    OPENVPN_PID=$(pgrep -f "openvpn" | head -1)
    echo "   PID: $OPENVPN_PID"
else
    echo "   ✗ OpenVPN is not running"
    exit 1
fi
echo ""

# Test 2: Check TUN interface
echo "2. Checking TUN interface..."
if ip link show tun0 >/dev/null 2>&1; then
    echo "   ✓ tun0 interface exists"
    VPN_IP=$(ip addr show tun0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)
    echo "   VPN IP: $VPN_IP"
else
    echo "   ✗ tun0 interface not found"
    exit 1
fi
echo ""

# Test 3: Check routing
echo "3. Checking routing table..."
DEFAULT_ROUTE=$(ip route | grep default || echo "No default route")
echo "   Default route: $DEFAULT_ROUTE"
VPN_ROUTES=$(ip route | grep "via.*tun0" | wc -l)
echo "   Routes via tun0: $VPN_ROUTES"
if [ "$VPN_ROUTES" -gt 0 ]; then
    echo "   ✓ Traffic is routed through VPN"
else
    echo "   ⚠ Warning: No routes via tun0 found"
fi
echo ""

# Test 4: Get public IP (direct, no proxy) - should show VPN IP
echo "4. Testing direct connection (should show VPN IP)..."
DIRECT_IP=$(curl -s --max-time 10 https://api.ipify.org 2>/dev/null || echo "Failed")
if [ "$DIRECT_IP" != "Failed" ] && [ -n "$DIRECT_IP" ]; then
    echo "   Direct IP: $DIRECT_IP"
    echo "   ✓ Direct connection working"
    
    # Verify it's not the host's public IP (basic check)
    if [ -n "$VPN_IP" ]; then
        echo "   Note: This IP should be different from your host's public IP"
    fi
else
    echo "   ✗ Direct connection failed"
fi
echo ""

# Test 5: Get public IP via Privoxy proxy
echo "5. Testing connection via Privoxy proxy..."
if pgrep privoxy >/dev/null; then
    PROXY_IP=$(curl -s --proxy http://127.0.0.1:8888 --max-time 10 https://api.ipify.org 2>/dev/null || echo "Failed")
    if [ "$PROXY_IP" != "Failed" ]; then
        echo "   Proxy IP: $PROXY_IP"
        echo "   ✓ Proxy connection working"
        
        # Compare IPs
        if [ "$DIRECT_IP" == "$PROXY_IP" ] && [ "$DIRECT_IP" != "Failed" ]; then
            echo "   ✓ Both direct and proxy show same IP (VPN is working)"
        elif [ "$DIRECT_IP" != "Failed" ] && [ "$PROXY_IP" != "Failed" ]; then
            echo "   ⚠ Warning: Direct and proxy IPs differ"
        fi
    else
        echo "   ✗ Proxy connection failed"
    fi
else
    echo "   ✗ Privoxy is not running"
fi
echo ""

# Test 6: Test DNS resolution
echo "6. Testing DNS resolution..."
if nslookup google.com >/dev/null 2>&1; then
    echo "   ✓ DNS resolution working"
else
    echo "   ⚠ DNS resolution test failed (may be normal)"
fi
echo ""

# Test 7: Test HTTP connectivity
echo "7. Testing HTTP connectivity..."
if curl -s --max-time 5 http://www.google.com >/dev/null 2>&1; then
    echo "   ✓ HTTP connectivity working"
else
    echo "   ✗ HTTP connectivity failed"
fi
echo ""

# Test 8: Test HTTPS connectivity via proxy
echo "8. Testing HTTPS via proxy..."
if curl -s --proxy http://127.0.0.1:8888 --max-time 10 https://www.google.com >/dev/null 2>&1; then
    echo "   ✓ HTTPS via proxy working"
else
    echo "   ✗ HTTPS via proxy failed"
fi
echo ""

# Summary
echo "=== Test Summary ==="
if [ "$DIRECT_IP" != "Failed" ] && [ -n "$VPN_IP" ]; then
    echo "✓ VPN is connected and working"
    echo "  Container VPN IP: $VPN_IP"
    echo "  Public IP (via VPN): $DIRECT_IP"
    if [ "$PROXY_IP" != "Failed" ]; then
        echo "  Proxy IP: $PROXY_IP"
    fi
    echo ""
    echo "To test from host:"
    echo "  curl --proxy http://<host-ip>:8888 https://api.ipify.org"
else
    echo "✗ VPN connection test failed"
    exit 1
fi
