# VPN Firewall Setup Scripts

## Host-Level Firewall (Required for Network Blocking)

To completely block direct internet access from containers on the `vpn_network`, you must run the host-level firewall script:

```bash
sudo ./scripts/setup-vpn-firewall.sh
```

This script:
- Blocks all outbound internet traffic from the `vpn_network` subnet
- Allows access to the VPN container (for proxy access)
- Allows DNS (port 53) for name resolution
- Allows local network access (10.x, 172.16-31.x, 192.168.x)

### What it does:

1. Creates an iptables chain `DOCKER-VPN-FILTER`
2. Adds rules to block direct internet access from VPN network
3. Allows only proxy access through the VPN container

### To remove the firewall rules:

```bash
VPN_SUBNET=$(docker network inspect vpn_network --format '{{range .IPAM.Config}}{{.Subnet}}{{end}}' | head -1 | grep -E '^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+/[0-9]+')
sudo iptables -D FORWARD -s $VPN_SUBNET ! -d $VPN_SUBNET -j DOCKER-VPN-FILTER
sudo iptables -F DOCKER-VPN-FILTER
sudo iptables -X DOCKER-VPN-FILTER
```

## Container-Level Configuration

The VPN container also has internal firewall setup, but due to Docker's network architecture, host-level rules are required for complete blocking.

## Current Status

- ✅ All containers have `HTTP_PROXY` and `HTTPS_PROXY` environment variables set
- ✅ Applications that respect these variables will use the proxy
- ⚠️ Network-level blocking requires running the host script above
- ⚠️ VPN connection not yet established (TLS handshake issue - separate from proxy)
