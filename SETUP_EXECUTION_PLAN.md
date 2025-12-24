# Cluster Setup Execution Plan

Follow these steps in order to set up your complete cluster with Gitea and CI/CD.

## Phase 1: Cleanup and Preparation

### Step 1.1: Clean Local Machine
```bash
# Stop local containers
docker stop pia-vpn 2>/dev/null || true
docker rm pia-vpn 2>/dev/null || true

# Note: Cluster cleanup will happen when we reconnect
```

### Step 1.2: Verify Environment
```bash
# Ensure environment variables are loaded
direnv allow

# Verify SMB configuration
halvor config show | grep SMB
```

## Phase 2: Remote Server Setup

### Step 2.1: Setup SMB Mounts on frigg
```bash
halvor smb frigg
```

This will mount:
- `/mnt/smb/maple/backups` → Backups (primary)
- `/mnt/smb/maple/data` → Data (primary, shared - must exist)
- `/mnt/smb/maple/halvor` → Halvor container data (primary)
- `/mnt/smb/willow/backups` → Backups (fallback)
- `/mnt/smb/willow/data` → Data (fallback, shared - must exist)
- `/mnt/smb/willow/halvor` → Halvor container data (fallback)

### Step 2.2: Setup SMB Mounts on baulder
```bash
halvor smb baulder
```

### Step 2.3: Verify SMB Mounts
```bash
# Check mounts on frigg
halvor ssh frigg "mount | grep smb"

# Check mounts on baulder
halvor ssh baulder "mount | grep smb"
```

## Phase 3: Cluster Provisioning

### Step 3.1: Provision frigg (Primary Control Plane)
```bash
halvor provision frigg --cluster-role control-plane
```

### Step 3.2: Provision baulder (HA Control Plane)
```bash
halvor provision baulder --cluster-role control-plane
```

### Step 3.3: Provision oak (Tiebreaker)
```bash
halvor provision oak --cluster-role tiebreaker
```

### Step 3.4: Verify Cluster
```bash
halvor k3s status -H frigg
# Should show all three nodes: frigg, baulder, oak
```

## Phase 4: Deploy Infrastructure

### Step 4.1: Deploy SMB Storage
```bash
# Note: -H frigg ensures deployment to cluster, not local machine
halvor install smb-storage --helm -H frigg
```

**Important**: Always use `-H frigg` (or `-H baulder`) for Helm chart deployments to ensure they go to the cluster, not your local machine.

This creates:
- PersistentVolumes for all SMB shares
- StorageClass: `smb-storage`
- Shared PVC: `halvor-shared-data`

### Step 4.2: Verify SMB Storage
```bash
kubectl get pv | grep smb
kubectl get pvc -n default | grep halvor-shared-data
```

## Phase 5: Deploy Gitea

### Step 5.1: Deploy Gitea
```bash
# Deploy to cluster (frigg is primary control plane)
halvor install gitea --helm -H frigg
```

**Note**: If you omit `-H frigg`, the command will now default to `frigg` for Helm charts, but it's better to be explicit.

### Step 5.2: Wait for Gitea to be Ready
```bash
kubectl wait --for=condition=available --timeout=10m deployment/gitea -n default
kubectl get pods -n default -l app.kubernetes.io/name=gitea
```

### Step 5.3: Access Gitea
1. Get Gitea URL from ingress:
   ```bash
   kubectl get ingress -n default gitea -o jsonpath='{.spec.rules[0].host}'
   ```
2. Access Gitea in browser
3. Complete initial setup (create admin user)

## Phase 6: Configure Gitea Repository

### Step 6.1: Create Repository in Gitea
1. Login to Gitea
2. Create new repository: `halvor`
3. Copy repository URL

### Step 6.2: Add Gitea as Remote
```bash
cd /Users/scottkey/code/halvor
./scripts/setup-gitea-remote.sh
```

Or manually:
```bash
git remote add gitea <gitea-repo-url>
git push gitea main
```

## Phase 7: Configure CI/CD

### Step 7.1: Setup Gitea Actions Secrets
In Gitea, go to Settings → Secrets and add:

1. **KUBECONFIG**: 
   ```bash
   # Get kubeconfig from cluster
   kubectl config view --flatten | base64
   # Paste the base64 encoded output
   ```

2. **GITHUB_TOKEN**: 
   - Create GitHub Personal Access Token with `repo` permissions
   - Paste token

3. **GITEA_DOMAIN**: 
   - Your Gitea domain (e.g., `gitea.scottkey.me`)

4. **GITEA_ROOT_URL**: 
   - Full Gitea URL (e.g., `https://gitea.scottkey.me`)

### Step 7.2: Create Experimental Branch
```bash
git checkout -b experimental
git push gitea experimental
```

This will trigger the `deploy-experimental.yml` workflow which will:
- Deploy all services to the cluster
- Use experimental image tags
- Wait for deployments to be ready

### Step 7.3: Verify CI/CD
1. Check Gitea Actions tab for workflow runs
2. Verify deployments in cluster:
   ```bash
   kubectl get deployments -n default
   kubectl get pods -n default
   ```

## Phase 8: Code Sync to GitHub

### Step 8.1: Verify Sync Workflow
The `sync-to-github.yml` workflow will automatically:
- Sync `experimental` branch to GitHub `experimental` branch
- Sync `main` branch to GitHub `main` branch
- Update experimental release tag on GitHub

### Step 8.2: Test Sync
```bash
# Make a change and push to experimental
git checkout experimental
# Make changes
git commit -am "Test CI/CD"
git push gitea experimental

# This will:
# 1. Trigger deployment workflow
# 2. Trigger sync to GitHub workflow
```

## Phase 9: Release Management

### Step 9.1: Create Official Release
```bash
# Tag a release
git tag v1.0.0
git push gitea v1.0.0

# This triggers release.yml workflow which:
# 1. Pushes tag to GitHub
# 2. Creates GitHub release
```

### Step 9.2: Manual Release (via Gitea UI)
1. Go to Gitea Actions
2. Run "Release to GitHub" workflow manually
3. Enter tag name (e.g., `v1.0.0`)

## Troubleshooting

### Cluster Unreachable
```bash
# Get kubeconfig from frigg
halvor k3s kubeconfig -H frigg > ~/.kube/config
export KUBECONFIG=~/.kube/config
```

### SMB Mounts Not Working
```bash
# Check SMB mounts
halvor ssh frigg "mount | grep smb"
halvor ssh baulder "mount | grep smb"

# Re-setup if needed
halvor smb frigg
halvor smb baulder
```

### Gitea Not Accessible
```bash
# Check ingress
kubectl get ingress -n default

# Check Traefik
kubectl get pods -n traefik
```

## Next Steps After Setup

1. **Deploy Additional Services**: Use `halvor install <service> --helm`
2. **Monitor Deployments**: `kubectl get pods -A`
3. **View Logs**: `kubectl logs -f deployment/<name> -n default`
4. **Update Services**: Push to experimental branch triggers auto-deploy

