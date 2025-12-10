# Usage Guide

## SSH to a Host

After setup, simply use standard SSH:

```bash
ssh maple
ssh bellerophon
```

With additional SSH arguments:

```bash
ssh maple -L 8080:localhost:8080
```

## Setup SMB Mounts

Setup and mount SMB shares on a remote host:

```bash
hal smb bellerophon
```

This will:

- Install SMB client utilities (`cifs-utils`)
- Create mount points at `/mnt/smb/{servername}/{sharename}`
- Mount SMB shares using credentials from `.env`
- Add entries to `/etc/fstab` for persistent mounts

**Uninstall SMB mounts:**

```bash
hal smb bellerophon --uninstall
```

## Backup and Restore Docker Volumes

**Create a backup:**

```bash
hal backup bellerophon create
```

This creates a timestamped backup of all Docker volumes and bind mounts in
`/mnt/smb/maple/backups/{hostname}/{timestamp}/`.

**List available backups:**

```bash
hal backup bellerophon list
```

**Restore from a backup:**

```bash
hal backup bellerophon restore
```

If no backup name is specified, it will list available backups and prompt you to select one.

Or restore a specific backup:

```bash
hal backup bellerophon restore --backup 20240101_120000
```

## Automatically Setup Nginx Proxy Manager Hosts

Automatically create proxy hosts in Nginx Proxy Manager from a Docker Compose file:

```bash
hal npm bellerophon media.docker-compose.yml
```

This will:

- Parse the compose file to find services with exposed ports
- Connect to Nginx Proxy Manager API (requires `NPM_USERNAME` and `NPM_PASSWORD` in `.env`)
- Create proxy hosts for each service (e.g., `sonarr.local`, `radarr.local`)
- Forward traffic to the host where services are running

The command will:

- Skip services that already have proxy hosts configured
- Use the host's IP or Tailscale address for forwarding
- Create domains in the format `{servicename}.local`

## VPN Deployment

Build and deploy VPN containers:

```bash
hal vpn build
hal vpn deploy
```

See the [VPN documentation](vpn.md) for more details.

## Updates

HAL can check for and install updates automatically.

### Check for Updates

Check for updates and install if available:

```bash
hal update
```

This will:

- Check for a newer version on GitHub releases
- Prompt you to download and install if an update is available
- Install the update automatically if you confirm

### Experimental Channel

Use the experimental channel to get the latest continuously-updated builds:

```bash
hal update --beta
```

This will:

- Check for updates from the experimental release (versionless, continuously updated)
- Always considers experimental releases as "newer" since they're versionless
- Useful for testing the latest code from the main branch

**Note:** Experimental releases are automatically created when code is merged to main and may be
unstable.

### Force Update

Force download and install the latest version (useful for development):

```bash
hal update --force
hal update --force --beta
```

This will:

- Skip the version check
- Download and install the latest release regardless of your current version
- Use `--beta` flag to force install from experimental channel
- Useful when you want to reinstall the latest version during development
