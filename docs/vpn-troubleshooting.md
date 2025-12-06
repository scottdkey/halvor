# VPN Container Troubleshooting

## "No OpenVPN config file found in /config" Error

If you're seeing this error when deploying the VPN container via Portainer, check the following:

### 1. Set USER Environment Variable

**Important**: The compose file uses `/home/${USER}/config/vpn`. You must set the `USER` environment variable in Portainer to match the username on the host.

In Portainer:
1. Go to your stack
2. Click "Editor" or "Environment"
3. Add environment variable: `USER=testUser` (replace with your actual username)
4. Redeploy the stack

### 2. Verify Files Exist on Host

SSH into the host and verify the files exist:

```bash
# Replace '${USER}' with your actual username, or will run as the current user
ls -la /home/${USER}/config/vpn/
```

You should see:
- `ca-montreal.ovpn` (or another `.ovpn` file)
- `auth.txt`

### 2. Check Directory Permissions

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

### 3. Verify Docker Can Access the Directory

Test if Docker can access the directory:

```bash
# Run a test container to check access (use your actual home path)
docker run --rm -v $HOME/config/vpn:/test:ro alpine ls -la /test
```

If this fails, Docker doesn't have access to the directory.

### 4. Portainer-Specific Issues

If deploying via Portainer:

- **Portainer Agent**: The agent runs on the host and should have access to host filesystem
- **Portainer CE**: If running in a container, ensure it has access to host volumes
- **SELinux**: On systems with SELinux, you may need to add `:z` or `:Z` to volume mounts:
  ```yaml
  volumes:
    - ${HOME}/config/vpn:/config:ro,z
  ```

### 5. Alternative: Use UPDATE_CONFIGS

If you can't fix the mount issue, enable automatic config download:

```yaml
environment:
  - UPDATE_CONFIGS=true
volumes:
  # Remove :ro to allow writing
  - ${HOME}/config/vpn:/config
```

This will download PIA configs automatically on startup.

### 6. Check Container Logs

View detailed error messages:

```bash
docker logs openvpn-pia
```

The entrypoint script now provides detailed debugging information about directory access.
