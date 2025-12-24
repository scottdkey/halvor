# Routing Traffic Through PIA VPN in Kubernetes

This guide explains how to route traffic from other pods in your Kubernetes cluster through the PIA VPN service.

## Overview

The PIA VPN Helm chart deploys a VPN proxy service that other pods can use to route their traffic through. The service exposes an HTTP proxy on port 8888 that routes traffic through the Private Internet Access VPN.

## Installation

Install the PIA VPN chart:

```bash
halvor helm install pia-vpn
```

Or using Helm directly:

```bash
helm install pia-vpn ./charts/pia-vpn \
  --set credentials.username="your_pia_username" \
  --set credentials.password="your_pia_password" \
  --set vpn.region="us-california"
```

## Using the VPN Proxy

### Method 1: Environment Variables in Pods

Configure your pods to use the VPN proxy by setting the `HTTP_PROXY` and `HTTPS_PROXY` environment variables:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  template:
    spec:
      containers:
      - name: my-app
        image: my-app:latest
        env:
        - name: HTTP_PROXY
          value: "http://pia-vpn:8888"
        - name: HTTPS_PROXY
          value: "http://pia-vpn:8888"
        - name: NO_PROXY
          value: "localhost,127.0.0.1,.svc,.svc.cluster.local"
```

### Method 2: Using a Sidecar Container

For applications that don't support proxy environment variables, you can use a sidecar container that routes traffic:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app
spec:
  template:
    spec:
      containers:
      - name: my-app
        image: my-app:latest
        # Your app configuration
      - name: vpn-proxy
        image: curlimages/curl:latest
        command: ["/bin/sh", "-c"]
        args:
        - |
          while true; do
            curl --proxy http://pia-vpn:8888 https://api.ipify.org
            sleep 60
          done
        env:
        - name: HTTP_PROXY
          value: "http://pia-vpn:8888"
        - name: HTTPS_PROXY
          value: "http://pia-vpn:8888"
```

### Method 3: Network Policy Routing

The PIA VPN chart includes a NetworkPolicy that allows traffic from all pods in the cluster. To route specific traffic through the VPN, you can:

1. **Use the VPN service as a gateway**: Configure your application to use `http://pia-vpn:8888` as a proxy
2. **Use iptables rules**: For more advanced routing, you can use init containers to set up iptables rules

## Service Discovery

The VPN service is available at:
- **Service Name**: `pia-vpn` (or `{{ release-name }}-pia-vpn` if using a custom release name)
- **Namespace**: The namespace where the chart is installed
- **Port**: `8888` (HTTP proxy)

## Testing the VPN

You can test the VPN connection from within the cluster:

```bash
# Get a shell in a pod
kubectl run -it --rm test-pod --image=curlimages/curl:latest --restart=Never -- sh

# Test direct connection (should show your real IP)
curl https://api.ipify.org

# Test via VPN proxy (should show VPN IP)
curl --proxy http://pia-vpn:8888 https://api.ipify.org
```

## Network Policy

The chart includes a NetworkPolicy that:
- Allows ingress from all pods in the cluster (if `networkPolicy.allowFromAll` is true)
- Allows egress to all destinations (required for VPN connectivity)

To restrict access to specific namespaces:

```yaml
networkPolicy:
  enabled: true
  allowFromAll: false
  allowedNamespaces:
    - media
    - downloads
```

## Troubleshooting

### VPN Not Connecting

1. Check the VPN pod logs:
   ```bash
   kubectl logs -l app.kubernetes.io/name=pia-vpn
   ```

2. Verify credentials are set correctly:
   ```bash
   kubectl get secret pia-vpn-credentials -o yaml
   ```

3. Check if the VPN service is running:
   ```bash
   kubectl get pods -l app.kubernetes.io/name=pia-vpn
   ```

### Proxy Not Working

1. Test the proxy from within the cluster:
   ```bash
   kubectl run -it --rm test --image=curlimages/curl:latest --restart=Never -- \
     curl --proxy http://pia-vpn:8888 https://api.ipify.org
   ```

2. Check service endpoints:
   ```bash
   kubectl get endpoints pia-vpn
   ```

3. Verify network policies:
   ```bash
   kubectl get networkpolicy
   ```

## Security Considerations

- **Credentials**: Store PIA credentials in Kubernetes secrets, not in plain text
- **Network Policy**: Restrict VPN access to only necessary namespaces
- **Service Account**: The VPN pod runs with elevated privileges (NET_ADMIN, NET_RAW) - ensure proper RBAC

## Example: Media Services Using VPN

Here's an example of configuring a media service (like qBittorrent) to use the VPN:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: qbittorrent
spec:
  template:
    spec:
      containers:
      - name: qbittorrent
        image: lscr.io/linuxserver/qbittorrent:latest
        env:
        - name: HTTP_PROXY
          value: "http://pia-vpn:8888"
        - name: HTTPS_PROXY
          value: "http://pia-vpn:8888"
        - name: NO_PROXY
          value: "localhost,127.0.0.1,.svc,.svc.cluster.local"
```

This ensures all outbound traffic from qBittorrent is routed through the VPN.

