#!/bin/bash

# Script to check proxy connectivity and network status
# Usage: docker exec <container> check-proxy.sh

echo "=== Network Status ==="
echo ""
echo "Container Network Interfaces:"
ip addr show | grep -E "^[0-9]+:|inet " | sed 's/^/  /'
echo ""

echo "Routing Table:"
ip route | sed 's/^/  /'
echo ""

echo "OpenVPN Status:"
if pgrep -f "openvpn" >/dev/null; then
    echo "  ✓ OpenVPN is running"
    OPENVPN_PID=$(pgrep -f "openvpn" | head -1)
    echo "  PID: $OPENVPN_PID"
else
    echo "  ✗ OpenVPN is not running"
fi
echo ""

echo "TUN Interface (VPN):"
if ip link show tun0 >/dev/null 2>&1; then
    echo "  ✓ tun0 exists"
    ip addr show tun0 | grep "inet " | sed 's/^/    /'
    echo "  VPN IP: $(ip addr show tun0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1)"
else
    echo "  ✗ tun0 not found"
fi
echo ""

echo "Privoxy Status:"
if pgrep privoxy >/dev/null; then
    echo "  ✓ Privoxy is running"
    PRIVOXY_PID=$(pgrep privoxy | head -1)
    echo "  PID: $PRIVOXY_PID"
else
    echo "  ✗ Privoxy is not running"
fi
echo ""

echo "Port 8888 Listening:"
if command -v netstat >/dev/null 2>&1; then
    netstat -tlnp 2>/dev/null | grep 8888 | sed 's/^/  /'
elif command -v ss >/dev/null 2>&1; then
    ss -tlnp 2>/dev/null | grep 8888 | sed 's/^/  /'
else
    echo "  (netstat/ss not available)"
fi
echo ""

echo "Proxy Test:"
echo "  Testing local connection..."
if curl -s --proxy http://127.0.0.1:8888 --max-time 5 https://api.ipify.org >/dev/null 2>&1; then
    VPN_IP=$(curl -s --proxy http://127.0.0.1:8888 --max-time 5 https://api.ipify.org)
    echo "  ✓ Proxy is working"
    echo "  VPN IP (via proxy): $VPN_IP"
else
    echo "  ✗ Proxy connection failed"
fi
echo ""

echo "Public IP (direct, no proxy):"
PUBLIC_IP=$(curl -s --max-time 5 https://api.ipify.org 2>/dev/null || echo "Unable to determine")
echo "  $PUBLIC_IP"
echo ""

echo "=== Access Information ==="
echo "  From host: http://<host-ip>:8888"
echo "  From containers: http://openvpn-pia:8888"
echo "  Example: curl --proxy http://10.10.10.14:8888 https://api.ipify.org"
echo ""
