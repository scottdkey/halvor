# VPN Setup Guide

## Quick Setup

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

## Portainer Configuration

When deploying via Portainer, you **must** set the `USER` environment variable:

1. Go to your stack in Portainer
2. Click "Editor" or find "Environment variables"
3. Add: `USER=<your-username>`
   - Example: `USER=username`
4. This determines the path: `/home/<your-username>/config/vpn`

## Using `hal vpn deploy`

The `hal vpn deploy` command automatically:

- Creates `/home/<user>/config/vpn` directory
- Copies `auth.txt` and `ca-montreal.ovpn` from your local `openvpn/` directory
- Sets proper permissions

You can override the username by setting `VPN_USER` environment variable:

```bash
VPN_USER=username hal vpn deploy <hostname>
```

## Troubleshooting

If you see "No OpenVPN config file found in /config":

1. **Check USER is set**: Verify `USER` environment variable in Portainer matches your host username
2. **Check files exist**: SSH to host and run `ls -la /home/$USER/config/vpn/`
3. **Check permissions**: Ensure Docker can read the directory
4. **Enable auto-download**: Set `UPDATE_CONFIGS=true` and remove `:ro` from volume mount

See [VPN Troubleshooting Guide](vpn-troubleshooting.md) for more details.
