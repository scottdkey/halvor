#!/bin/bash
# Setup firewall rules to restrict traefik_private_network to local network and Tailnet only
# This must be run on the Docker HOST (not in a container)
# Run with: sudo ./setup-firewall.sh

set -e

echo "Setting up firewall to restrict traefik_private_network to local network and Tailnet..."
echo ""

# Get the private Traefik network subnet
PRIVATE_NETWORK_NAME="traefik_private_network"

# Get all subnets and extract IPv4
ALL_SUBNETS=$(docker network inspect $PRIVATE_NETWORK_NAME --format '{{range .IPAM.Config}}{{.Subnet}} {{end}}' 2>/dev/null)

if [ -z "$ALL_SUBNETS" ]; then
    echo "❌ Error: Could not find Docker network '$PRIVATE_NETWORK_NAME'"
    echo "   Make sure the private Traefik network exists: docker network ls"
    exit 1
fi

# Extract IPv4 subnet only
PRIVATE_NETWORK_SUBNET=$(echo "$ALL_SUBNETS" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+' | head -1)

if [ -z "$PRIVATE_NETWORK_SUBNET" ]; then
    echo "❌ Error: Could not determine IPv4 subnet for '$PRIVATE_NETWORK_NAME'"
    echo "   Found subnets: $ALL_SUBNETS"
    exit 1
fi

echo "Found private Traefik network: $PRIVATE_NETWORK_SUBNET"
echo ""

# Get Tailscale network range (typically 100.x.x.x/8)
TAILSCALE_RANGE="100.0.0.0/8"

echo "Setting up iptables rules..."
echo ""

# Create a custom chain for private Traefik network filtering
iptables -N DOCKER-TRAEFIK-PRIVATE-FILTER 2>/dev/null || iptables -F DOCKER-TRAEFIK-PRIVATE-FILTER

# Allow established connections
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -m state --state ESTABLISHED,RELATED -j ACCEPT

# Allow local network access (10.x, 172.16-31.x, 192.168.x)
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -s 10.0.0.0/8 -j ACCEPT
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -s 172.16.0.0/12 -j ACCEPT
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -s 192.168.0.0/16 -j ACCEPT

# Allow Tailscale network access (100.x.x.x)
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -s $TAILSCALE_RANGE -j ACCEPT

# Block all other external access to the private network
iptables -A DOCKER-TRAEFIK-PRIVATE-FILTER -j DROP

# Apply the filter to traffic going TO the private network from external sources
# This goes in the FORWARD chain (Docker uses FORWARD for container traffic)
iptables -I FORWARD ! -s $PRIVATE_NETWORK_SUBNET -d $PRIVATE_NETWORK_SUBNET -j DOCKER-TRAEFIK-PRIVATE-FILTER

# Also block direct access to the private Traefik ports from external sources
# Allow only from local networks and Tailscale
iptables -I INPUT -p tcp --dport 8081 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP
iptables -I INPUT -p tcp --dport 8443 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP
iptables -I INPUT -p tcp --dport 8082 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP

echo "✓ Firewall rules configured"
echo ""
echo "Rules applied:"
echo "  - Allow established connections"
echo "  - Allow local network access (10.x, 172.16-31.x, 192.168.x)"
echo "  - Allow Tailscale network access (100.x.x.x)"
echo "  - BLOCK all other external access to private network"
echo "  - BLOCK external access to private Traefik ports (8081, 8443, 8082)"
echo ""
echo "To remove these rules, run:"
echo "  sudo iptables -D FORWARD ! -s $PRIVATE_NETWORK_SUBNET -d $PRIVATE_NETWORK_SUBNET -j DOCKER-TRAEFIK-PRIVATE-FILTER"
echo "  sudo iptables -D INPUT -p tcp --dport 8081 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP"
echo "  sudo iptables -D INPUT -p tcp --dport 8443 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP"
echo "  sudo iptables -D INPUT -p tcp --dport 8082 ! -s 10.0.0.0/8 ! -s 172.16.0.0/12 ! -s 192.168.0.0/16 ! -s $TAILSCALE_RANGE -j DROP"
echo "  sudo iptables -F DOCKER-TRAEFIK-PRIVATE-FILTER"
echo "  sudo iptables -X DOCKER-TRAEFIK-PRIVATE-FILTER"

