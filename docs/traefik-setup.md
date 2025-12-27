# Traefik Dual-Ingress Setup

This document describes how to set up dual Traefik ingress controllers for public and private traffic routing using a single unified chart.

## Architecture

```
                    Internet
                        │
                        ▼
┌─────────────────────────────────────┐
│         OAK (Public IP)             │
│                                     │
│   traefik-public                    │
│   - LoadBalancer on port 443        │
│   - IngressClass: traefik-public    │
│   - Only handles explicitly public  │
│     services                        │
└─────────────────────────────────────┘

                  Tailscale
                      │
                      ▼
┌─────────────────────────────────────┐
│         FRIGG (Tailscale IP)        │
│                                     │
│   traefik-private (DEFAULT)         │
│   - LoadBalancer on Tailscale IP    │
│   - IngressClass: traefik-private   │
│   - Handles ALL services by default │
└─────────────────────────────────────┘

                      │
                      ▼
┌─────────────────────────────────────┐
│      Tailscale Operator             │
│   - All services accessible via     │
│     tailscale-operator context      │
└─────────────────────────────────────┘
```

## Prerequisites

1. K3s cluster with oak and frigg nodes
2. Tailscale operator installed
3. Cloudflare API token for DNS validation (for Let's Encrypt certs)
4. Environment variables:
   - `PUBLIC_DOMAIN` - Your public domain (e.g., example.com)
   - `PRIVATE_DOMAIN` - Your private domain (e.g., home.example.com)
   - `ACME_EMAIL` - Email for Let's Encrypt
   - `CF_DNS_API_TOKEN` - Cloudflare DNS API token

## Installation

### 1. Install Traefik CRDs (one-time)

```bash
kubectl apply -f https://raw.githubusercontent.com/traefik/traefik/v3.2/docs/content/reference/dynamic-configuration/kubernetes-crd-definition-v1.yml
```

### 2. Install traefik-private (default ingress on frigg)

```bash
helm upgrade --install traefik-private ./charts/traefik \
  --namespace traefik \
  --create-namespace \
  -f ./charts/traefik/values-private.yaml \
  --set domain="${PRIVATE_DOMAIN}" \
  --set acme.email="${ACME_EMAIL}" \
  --set acme.dnsToken="${CF_DNS_API_TOKEN}" \
  --set dashboard.domain="traefik.${PRIVATE_DOMAIN}"
```

### 3. Install traefik-public (public ingress on oak)

```bash
helm upgrade --install traefik-public ./charts/traefik \
  --namespace traefik \
  -f ./charts/traefik/values-public.yaml \
  --set domain="${PUBLIC_DOMAIN}" \
  --set acme.email="${ACME_EMAIL}" \
  --set acme.dnsToken="${CF_DNS_API_TOKEN}" \
  --set dashboard.domain="traefik.${PUBLIC_DOMAIN}"
```

## Chart Configuration

The unified `traefik` chart supports two modes:

| Setting | Public Mode | Private Mode |
|---------|-------------|--------------|
| `mode` | `public` | `private` |
| `ingressClass.name` | `traefik-public` | `traefik-private` |
| `ingressClass.isDefault` | `false` | `true` |
| `node.hostname` | `oak` | `frigg` |

### Key Values

```yaml
# Mode determines defaults for ingress class and node placement
mode: private  # or "public"

# Override auto-generated ingress class name
ingressClass:
  name: ""  # defaults to traefik-{mode}
  isDefault: null  # defaults based on mode

# Override node placement
node:
  hostname: ""  # defaults to oak (public) or frigg (private)

# ACME configuration
acme:
  enabled: true
  email: "admin@example.com"
  dnsProvider: cloudflare
  dnsToken: "your-token"

# Domain for this ingress
domain: "example.com"
```

## Usage

### Default (Private) Service

Services without an explicit `ingressClassName` route through `traefik-private`:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-app
  annotations:
    traefik.ingress.kubernetes.io/router.tls.certresolver: cloudflare
spec:
  # No ingressClassName = uses default (traefik-private)
  rules:
  - host: my-app.home.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: my-app
            port:
              number: 80
  tls:
  - hosts:
    - my-app.home.example.com
```

### Public Service

Expose a service publicly via oak's public IP:

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-public-app
  annotations:
    traefik.ingress.kubernetes.io/router.tls.certresolver: cloudflare
spec:
  ingressClassName: traefik-public  # <-- Explicitly public
  rules:
  - host: my-app.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: my-app
            port:
              number: 80
  tls:
  - hosts:
    - my-app.example.com
```

### Using IngressRoute CRD

For more control, use Traefik's IngressRoute CRD:

```yaml
apiVersion: traefik.io/v1alpha1
kind: IngressRoute
metadata:
  name: my-app
  annotations:
    kubernetes.io/ingress.class: traefik-private  # or traefik-public
spec:
  entryPoints:
    - websecure
  routes:
  - match: Host(`my-app.home.example.com`)
    kind: Rule
    services:
    - name: my-app
      port: 80
  tls:
    certResolver: cloudflare
```

## Verification

```bash
# Check ingress classes
kubectl get ingressclass

# Check traefik pods
kubectl get pods -n traefik

# Check services (should show LoadBalancer IPs)
kubectl get svc -n traefik

# View traefik logs
kubectl logs -n traefik -l app.kubernetes.io/name=traefik
```

## Switching kubectl context

```bash
# Use Tailscale operator (works from anywhere)
halvor k8s tailscale

# Use direct connection (faster, requires network access)
halvor k8s direct
```
