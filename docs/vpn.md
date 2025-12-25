# VPN Guide

Complete guide for setting up, routing traffic through, and troubleshooting the PIA VPN container in Kubernetes.

## Overview

The PIA VPN Helm chart deploys a VPN proxy service that other pods can use to route their traffic through. The service exposes an HTTP proxy on port 8888 that routes traffic through the Private Internet Access VPN.

## Installation

Install the PIA VPN chart:

```bash
halvor install pia-vpn -H <cluster-node>
```

The CLI automatically detects that pia-vpn is a Helm chart and deploys it to the Kubernetes cluster.

Or using Helm directly:

```bash
helm install pia-vpn ./charts/pia-vpn \
  --set credentials.username="your_pia_username" \
  --set credentials.password="your_pia_password" \
  --set vpn.region="us-california"
```

## Setup

The VPN container needs OpenVPN configuration files. You have two options:

### Option 1: Auto-Download Configs (Recommended)

Enable automatic download of PIA configs on startup:

1. **In Portainer**, edit your stack and:
   - Set environment variable: `UPDATE_CONFIGS=true`
   - Change the volume mount from `:ro` to writable:
     ```yaml
     volumes:
       - /home/${USER}/config/vpn:/config
     ```
   - Set `USER` environment variable to your username (e.g., `USER=username`)

2. **Redeploy the stack**

The container will automatically download PIA OpenVPN configs on first startup.

### Option 2: Manual File Deployment

If you prefer to deploy files manually:

1. **SSH into the host** and create the directory:

   ```bash
   mkdir -p ~/config/vpn
   ```

2. **Copy your OpenVPN files**:

   ```bash
   # Copy your .ovpn config file
   cp ca-montreal.ovpn ~/config/vpn/

   # Create auth.txt with PIA credentials
   cat > ~/config/vpn/auth.txt << EOF
   your-pia-username
   your-pia-password
   EOF

   # Set proper permissions
   chmod 644 ~/config/vpn/*.ovpn
   chmod 600 ~/config/vpn/auth.txt
   ```

3. **In Portainer**, ensure:
   - `USER` environment variable is set to your username
   - Volume mount uses `:ro` (read-only) since files are pre-deployed

### Portainer Configuration

When deploying via Portainer, you **must** set the `USER` environment variable:

1. Go to your stack in Portainer
2. Click "Editor" or find "Environment variables"
3. Add: `USER=<your-username>`
   - Example: `USER=username`
4. This determines the path: `/home/<your-username>/config/vpn`

## Routing Traffic Through VPN

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

## Troubleshooting

### "No OpenVPN config file found in /config" Error

If you're seeing this error when deploying the VPN container, check the following:

#### 1. Set USER Environment Variable

**Important**: The compose file uses `/home/${USER}/config/vpn`. You must set the `USER` environment variable in Portainer to match the username on the host.

In Portainer:

1. Go to your stack
2. Click "Editor" or "Environment"
3. Add environment variable: `USER=<your-username>` (replace with your actual username)
4. Redeploy the stack

#### 2. Verify Files Exist on Host

SSH into the host and verify the files exist:

```bash
# Replace '${USER}' with your actual username, or will run as the current user
ls -la /home/${USER}/config/vpn/
```

You should see:

- `ca-montreal.ovpn` (or another `.ovpn` file)
- `auth.txt`

#### 3. Check Directory Permissions

The directory should be in your home directory, so permissions should be fine:

```bash
# Check current permissions
ls -ld ~/config/vpn

# Ensure directory exists and has proper permissions
mkdir -p ~/config/vpn
chmod 755 ~/config/vpn
chmod 644 ~/config/vpn/*.ovpn 2>/dev/null || true
chmod 600 ~/config/vpn/auth.txt 2>/dev/null || true
```

#### 4. Verify Docker Can Access the Directory

Test if Docker can access the directory:

```bash
# Run a test container to check access (use your actual home path)
docker run --rm -v $HOME/config/vpn:/test:ro alpine ls -la /test
```

If this fails, Docker doesn't have access to the directory.

#### 5. Portainer-Specific Issues

If deploying via Portainer:

- **Portainer Agent**: The agent runs on the host and should have access to host filesystem
- **Portainer CE**: If running in a container, ensure it has access to host volumes
- **SELinux**: On systems with SELinux, you may need to add `:z` or `:Z` to volume mounts:
  ```yaml
  volumes:
    - ${HOME}/config/vpn:/config:ro,z
  ```

#### 6. Alternative: Use UPDATE_CONFIGS

If you can't fix the mount issue, enable automatic config download:

```yaml
environment:
  - UPDATE_CONFIGS=true
volumes:
  # Remove :ro to allow writing
  - ${HOME}/config/vpn:/config
```

This will download PIA configs automatically on startup.

#### 7. Check Container Logs

View detailed error messages:

```bash
docker logs openvpn-pia
```

The entrypoint script now provides detailed debugging information about directory access.

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

