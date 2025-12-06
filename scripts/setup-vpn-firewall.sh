#!/bin/bash
# Host-level script to block direct internet access from VPN network
# This must be run on the Docker HOST (not in a container)
# Run with: sudo ./scripts/setup-vpn-firewall.sh

set -e

echo "Setting up host-level firewall to block direct internet access from VPN network..."
echo ""

# Get the VPN network subnet
VPN_NETWORK_NAME="vpn_network"

# Get all subnets and extract IPv4
ALL_SUBNETS=$(docker network inspect $VPN_NETWORK_NAME --format '{{range .IPAM.Config}}{{.Subnet}} {{end}}' 2>/dev/null)

if [ -z "$ALL_SUBNETS" ]; then
    echo "❌ Error: Could not find Docker network '$VPN_NETWORK_NAME'"
    echo "   Make sure the VPN network exists: docker network ls"
    exit 1
fi

# Extract IPv4 subnet only (Docker may return IPv4 and IPv6)
VPN_NETWORK_SUBNET=$(echo "$ALL_SUBNETS" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+' | head -1)

if [ -z "$VPN_NETWORK_SUBNET" ]; then
    echo "❌ Error: Could not determine IPv4 subnet for '$VPN_NETWORK_NAME'"
    echo "   Found subnets: $ALL_SUBNETS"
    exit 1
fi

echo "Found VPN network: $VPN_NETWORK_SUBNET"
echo ""

# Get VPN container IP
VPN_CONTAINER_IP=$(docker inspect pia-vpn --format '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' 2>/dev/null)

if [ -z "$VPN_CONTAINER_IP" ]; then
    echo "⚠ Warning: Could not find pia-vpn container IP"
    echo "   Continuing anyway - rules will be applied when container starts"
else
    echo "VPN container IP: $VPN_CONTAINER_IP"
fi

echo ""
echo "Setting up iptables rules..."

# Create a custom chain for VPN network filtering
iptables -N DOCKER-VPN-FILTER 2>/dev/null || iptables -F DOCKER-VPN-FILTER

# Allow established connections
iptables -A DOCKER-VPN-FILTER -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow access to VPN container (for proxy)
if [ -n "$VPN_CONTAINER_IP" ]; then
    iptables -A DOCKER-VPN-FILTER -d $VPN_CONTAINER_IP -j ACCEPT
fi

# Allow DNS (port 53) - needed for name resolution
iptables -A DOCKER-VPN-FILTER -p udp --dport 53 -j ACCEPT
iptables -A DOCKER-VPN-FILTER -p tcp --dport 53 -j ACCEPT

# Allow local network access (10.x, 172.16-31.x, 192.168.x)
iptables -A DOCKER-VPN-FILTER -d 10.0.0.0/8 -j ACCEPT
iptables -A DOCKER-VPN-FILTER -d 172.16.0.0/12 -j ACCEPT
iptables -A DOCKER-VPN-FILTER -d 192.168.0.0/16 -j ACCEPT

# Block all other outbound traffic from VPN network
iptables -A DOCKER-VPN-FILTER -j DROP

# Apply the filter to traffic from VPN network going to internet
# This goes in the FORWARD chain (Docker uses FORWARD for container traffic)
iptables -I FORWARD -s $VPN_NETWORK_SUBNET ! -d $VPN_NETWORK_SUBNET -j DOCKER-VPN-FILTER

echo "✓ Firewall rules configured"
echo ""
echo "Rules applied:"
echo "  - Allow established connections"
echo "  - Allow access to VPN container (proxy)"
echo "  - Allow DNS"
echo "  - Allow local network access"
echo "  - BLOCK all other internet access"
echo ""
echo "To remove these rules, run:"
echo "  sudo iptables -D FORWARD -s $VPN_NETWORK_SUBNET ! -d $VPN_NETWORK_SUBNET -j DOCKER-VPN-FILTER"
echo "  sudo iptables -F DOCKER-VPN-FILTER"
echo "  sudo iptables -X DOCKER-VPN-FILTER"
