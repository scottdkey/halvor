# Docker Compose to Helm Chart Migration Guide

This document outlines the migration of all Docker Compose services to Helm charts.

## Status

### Completed
- ✅ **PIA VPN** - Migrated to `charts/pia-vpn/` with full Kubernetes support
- ✅ **Traefik Public** - `charts/traefik-public/`
- ✅ **Traefik Private** - `charts/traefik-private/`
- ✅ **Gitea** - `charts/gitea/`
- ✅ **SMB Storage** - `charts/smb-storage/`

### In Progress
- 🔄 **Media Services** - Sonarr, Radarr, Bazarr, Prowlarr, qBittorrent, SABnzbd

### Pending
- ⏳ **Portainer** - `compose/portainer/`
- ⏳ **Portainer Agent** - `compose/portainer-agent/`
- ⏳ **Nginx Proxy Manager** - `compose/nginx-proxy-manager/`

## Migration Process

### 1. Create Helm Chart Structure

For each service, create:
```
charts/<service-name>/
├── Chart.yaml          # Chart metadata
├── values.yaml         # Default values
└── templates/
    ├── _helpers.tpl    # Template helpers
    ├── deployment.yaml # Main deployment
    ├── service.yaml    # Service definition
    ├── ingress.yaml    # Ingress (if needed)
    ├── pvc.yaml        # Persistent volumes (if needed)
    └── secret.yaml     # Secrets (if needed)
```

### 2. Convert Docker Compose to Kubernetes Resources

#### Deployment
- Convert `services.*.image` → `containers[].image`
- Convert `services.*.environment` → `containers[].env`
- Convert `services.*.volumes` → `containers[].volumeMounts` + `volumes`
- Convert `services.*.ports` → `service.ports`
- Convert `services.*.networks` → `service.type` or `ingress`

#### Persistent Volumes
- Convert `volumes.*` → `PersistentVolumeClaim`
- Use `storageClass` from SMB storage chart for SMB-backed volumes

#### Networks
- Convert `networks.*` → `Service` with appropriate `type`
- For VPN network, use `NetworkPolicy` to allow routing

### 3. Environment Variables

Load from 1Password via `halvor helm install`:
- Chart-specific variables (e.g., `SONARR_API_KEY`)
- Common variables (e.g., `PUBLIC_DOMAIN`, `PRIVATE_DOMAIN`)
- Credentials (stored as Kubernetes Secrets)

### 4. Update `halvor helm install`

Add chart-specific value generation in `src/services/helm/mod.rs`:
```rust
match chart {
    "sonarr" => {
        // Load Sonarr-specific env vars
        // Generate values
    }
    // ... other charts
}
```

## Service-Specific Notes

### Media Services (Sonarr, Radarr, Bazarr, Prowlarr)

**Common Pattern:**
- Use SMB storage for media files
- Connect to VPN network for privacy (via HTTP_PROXY env var)
- Expose web UI via Traefik ingress
- Use persistent volumes for config and data

**Example Values:**
```yaml
image:
  repository: lscr.io/linuxserver/sonarr
  tag: latest

persistence:
  config:
    enabled: true
    size: 10Gi
    storageClass: smb-storage
  downloads:
    enabled: true
    size: 100Gi
    storageClass: smb-storage

vpn:
  enabled: true
  proxy: "http://pia-vpn:8888"

ingress:
  enabled: true
  domain: "sonarr.scottkey.dev"
  traefik: "traefik-public"
```

### Portainer

**Notes:**
- Requires Docker socket access (hostPath volume)
- May need privileged mode
- Expose via Traefik

### Nginx Proxy Manager

**Notes:**
- Replaced by Traefik in most cases
- Keep for backward compatibility if needed
- Similar structure to Traefik charts

## VPN Integration

All services that need VPN routing should:

1. **Set HTTP_PROXY environment variable:**
   ```yaml
   env:
   - name: HTTP_PROXY
     value: "http://pia-vpn:8888"
   - name: HTTPS_PROXY
     value: "http://pia-vpn:8888"
   - name: NO_PROXY
     value: "localhost,127.0.0.1,.svc,.svc.cluster.local"
   ```

2. **Use NetworkPolicy to allow VPN access:**
   ```yaml
   networkPolicy:
     enabled: true
     allowVPN: true
   ```

## Migration Checklist

For each service:

- [ ] Create Helm chart structure
- [ ] Convert docker-compose.yml to Kubernetes resources
- [ ] Create values.yaml with defaults
- [ ] Add environment variable loading in `src/services/helm/mod.rs`
- [ ] Test installation: `halvor helm install <service>`
- [ ] Test upgrade: `halvor helm upgrade <service>`
- [ ] Test uninstall: `halvor helm uninstall <service>`
- [ ] Update documentation
- [ ] Remove docker-compose.yml (or mark as deprecated)

## Script Migration

All bash scripts in `compose/*/scripts/` should be:
1. **Incorporated into Rust code** - For container entrypoints (like PIA VPN)
2. **Converted to Kubernetes Jobs/CronJobs** - For periodic tasks
3. **Removed** - If functionality is replaced by Helm chart features

## Next Steps

1. Create Helm charts for media services (Sonarr, Radarr, etc.)
2. Create Helm chart for Portainer
3. Update `halvor install` command to use Helm instead of docker-compose
4. Remove or deprecate docker-compose files

