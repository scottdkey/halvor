# Helm Charts

This document lists all Helm charts available for installation via halvor.

## Available Helm Charts

Helm charts are automatically detected by halvor - no `--helm` flag is needed. Simply use:

```bash
halvor install <chart-name> -H <hostname>
```


## Charts

Helm Charts:
  portainer            - Portainer CE/BE/Agent - Container management UI (use deploymentType: ce/be/agent)
  nginx-proxy-manager  - Reverse proxy with SSL (aliases: npm, proxy)
  traefik-public       - Public Traefik reverse proxy (PUBLIC_DOMAIN from 1Password) (aliases: traefik-pub, traefik-dev)
  traefik-private      - Private Traefik reverse proxy (PRIVATE_DOMAIN from 1Password, local/Tailnet only) (aliases: traefik-priv, traefik-me)
  gitea                - Gitea Git hosting service (aliases: git)
  smb-storage          - SMB storage setup for Kubernetes (backups, data, docker-appdata) (aliases: smb, storage)
  pia-vpn              - PIA VPN with HTTP proxy (Kubernetes deployment) (aliases: pia, vpn)
  sabnzbd              - Usenet download client (aliases: sab) [requires vpn]
  qbittorrent          - Torrent download client (aliases: qbt, torrent) [requires vpn]
  radarr               - Movie management and automation [requires vpn]
  radarr-4k            - Movie management for 4K content (aliases: radarr4k) [requires vpn]
  sonarr               - TV show management and automation [requires vpn]
  prowlarr             - Indexer manager for *arr apps [requires vpn]
  bazarr               - Subtitle management [requires vpn]
  halvor-server        - Halvor server with web UI and agent API (aliases: halvor, server)

Usage:
  halvor install <app>                  # Install on current system
  halvor install <app> -H <hostname>    # Install on remote host

Note: Helm charts are automatically detected. No --helm flag needed.

## Notes

- All Helm charts are automatically detected - no `--helm` flag needed
- The CLI validates cluster availability before installing Helm charts
- Charts default to the `frigg` hostname if no `-H` option is provided
- Use `halvor install --list` to see all available apps
