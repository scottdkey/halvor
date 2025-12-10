# IPv6 Configuration for VPN Container

## Overview

The VPN container can be configured with IPv6 support to eliminate OpenVPN warnings about IPv6
routes. This requires configuration at both the Docker daemon and network levels.

## Prerequisites

1. **Docker daemon must have IPv6 enabled**
2. **Docker network must be configured with IPv6 subnet**
3. **Host system must have IPv6 connectivity**

## Step 1: Enable IPv6 in Docker Daemon

On the Docker host (`bellerophon`), configure the Docker daemon to enable IPv6:

### Edit Docker daemon configuration

```bash
sudo mkdir -p /etc/docker
sudo nano /etc/docker/daemon.json
```

Add or update the configuration:

```json
{
  "ipv6": true,
  "fixed-cidr-v6": "fd00:172:20::/64"
}
```

**Note:** `fd00:172:20::/64` is a ULA (Unique Local Address) that:

- Works without global IPv6 connectivity
- Matches the IPv4 subnet pattern (172.20.x.x)
- Is safe to use in private networks
- If you have global IPv6, you can use a subnet from your allocation instead

### Restart Docker daemon

```bash
sudo systemctl restart docker
```

### Verify IPv6 is enabled

```bash
docker info | grep -i ipv6
# Should show: IPv6: true
```

## Step 2: Update Docker Compose Network

The compose files already include IPv6 configuration, but you may need to adjust the subnet:

```yaml
networks:
  vpn_network:
    name: vpn_network
    driver: bridge
    enable_ipv6: true
    ipam:
      config:
        - subnet: 172.20.0.0/16
        - subnet: fd00:172:20::/64 # ULA matching IPv4 subnet pattern
```

**Note:** The IPv6 subnet in the compose file should match or be within the range specified in
`daemon.json`.

## Step 3: Recreate the Network

If the network already exists, you'll need to remove and recreate it:

```bash
# Stop all containers using the network
docker-compose -f compose/openvpn-pia-portainer.docker-compose.yml down

# Remove the existing network
docker network rm vpn_network

# Recreate with IPv6 enabled
docker-compose -f compose/openvpn-pia-portainer.docker-compose.yml up -d
```

## Step 4: Verify IPv6 in Container

After starting the container, verify IPv6 is available:

```bash
docker exec openvpn-pia ip -6 addr show
# Should show IPv6 addresses on network interfaces
```

The OpenVPN IPv6 warning should no longer appear.

## Troubleshooting

### "IPv6 not available" message

If you see this in container logs:

- Verify Docker daemon has IPv6 enabled: `docker info | grep IPv6`
- Check network configuration: `docker network inspect vpn_network | grep -i ipv6`
- Ensure the IPv6 subnet in compose matches daemon configuration

### IPv6 subnet conflicts

If you get subnet conflicts:

- Use a different ULA like `fd00:172:21::/64` or `fd01:172:20::/64`
- Ensure the subnet doesn't overlap with existing Docker networks
- Check existing networks: `docker network ls` and `docker network inspect <network>`

### OpenVPN still shows IPv6 warnings

Even with IPv6 enabled, you may see warnings if:

- The VPN server doesn't provide IPv6 connectivity
- IPv6 routes can't be established
- This is normal and doesn't affect IPv4 VPN functionality

## Alternative: Disable IPv6 Warnings

If you don't need IPv6, you can ignore the warnings - they don't affect VPN functionality. The
container will work fine with IPv4 only.
