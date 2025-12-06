#!/bin/bash
# Script to verify Privoxy is routing traffic through VPN

set -e

echo "=== Privoxy VPN Connection Test ==="
echo ""

# Detect local public IP (without VPN/proxy)
echo "1. Detecting your local public IP (direct connection)..."
LOCAL_IP=$(curl -s --max-time 10 https://api.ipify.org 2>/dev/null || curl -s --max-time 10 https://ifconfig.me 2>/dev/null || curl -s --max-time 10 https://icanhazip.com 2>/dev/null || echo "unknown")

if [ "$LOCAL_IP" = "unknown" ]; then
    echo "   ⚠ Could not detect local IP"
    exit 1
fi

echo "   ✓ Local public IP: $LOCAL_IP"
echo ""

# Check if Privoxy is accessible
echo "2. Testing Privoxy connection..."
PRIVOXY_HOST="${PRIVOXY_HOST:-localhost}"
PRIVOXY_PORT="${PRIVOXY_PORT:-8888}"

if curl -s --max-time 5 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" https://api.ipify.org > /dev/null 2>&1; then
    echo "   ✓ Privoxy is accessible on ${PRIVOXY_HOST}:${PRIVOXY_PORT}"
else
    echo "   ✗ Privoxy is NOT accessible on ${PRIVOXY_HOST}:${PRIVOXY_PORT}"
    echo "   Make sure the container is running and port 8888 is exposed"
    exit 1
fi
echo ""

# Test IP through Privoxy
echo "3. Testing IP address through Privoxy proxy..."
PROXY_IP=$(curl -s --max-time 10 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" https://api.ipify.org 2>/dev/null || curl -s --max-time 10 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" https://ifconfig.me 2>/dev/null || curl -s --max-time 10 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" https://icanhazip.com 2>/dev/null || echo "unknown")

if [ "$PROXY_IP" = "unknown" ]; then
    echo "   ✗ Could not determine IP through Privoxy"
    exit 1
fi

echo "   ✓ IP through Privoxy: $PROXY_IP"
echo ""

# Compare IPs
echo "4. Verifying VPN routing..."
if [ "$PROXY_IP" = "$LOCAL_IP" ]; then
    echo "   ✗ WARNING: IP address unchanged!"
    echo "   Local IP:  $LOCAL_IP"
    echo "   Proxy IP: $PROXY_IP"
    echo "   Privoxy is NOT routing through VPN"
    exit 1
else
    echo "   ✓ SUCCESS: IP address changed!"
    echo "   Local IP:  $LOCAL_IP"
    echo "   Proxy IP: $PROXY_IP"
    echo "   Privoxy is routing through VPN ✓"
fi
echo ""

# Test HTTP connection through proxy
echo "5. Testing HTTP connection through Privoxy..."
HTTP_TEST=$(curl -s --max-time 10 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" http://httpbin.org/ip 2>/dev/null | grep -o '"origin":"[^"]*"' | cut -d'"' -f4 || echo "failed")

if [ "$HTTP_TEST" != "failed" ] && [ "$HTTP_TEST" = "$PROXY_IP" ]; then
    echo "   ✓ HTTP connection working through Privoxy"
    echo "   Verified IP: $HTTP_TEST"
else
    echo "   ⚠ HTTP test inconclusive"
fi
echo ""

# Test HTTPS connection through proxy
echo "6. Testing HTTPS connection through Privoxy..."
HTTPS_TEST=$(curl -s --max-time 10 --proxy "http://${PRIVOXY_HOST}:${PRIVOXY_PORT}" https://httpbin.org/ip 2>/dev/null | grep -o '"origin":"[^"]*"' | cut -d'"' -f4 || echo "failed")

if [ "$HTTPS_TEST" != "failed" ] && [ "$HTTPS_TEST" = "$PROXY_IP" ]; then
    echo "   ✓ HTTPS connection working through Privoxy"
    echo "   Verified IP: $HTTPS_TEST"
else
    echo "   ⚠ HTTPS test inconclusive"
fi
echo ""

echo "=== Test Summary ==="
echo ""
echo "Local IP (direct):     $LOCAL_IP"
echo "Proxy IP (via VPN):    $PROXY_IP"
echo "Privoxy Status:        ✓ Working"
echo "VPN Routing:           ✓ Verified"
echo ""
echo "To use Privoxy in your applications:"
echo "  export HTTP_PROXY=http://${PRIVOXY_HOST}:${PRIVOXY_PORT}"
echo "  export HTTPS_PROXY=http://${PRIVOXY_HOST}:${PRIVOXY_PORT}"
echo ""
