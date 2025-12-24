# Complete Cluster Setup Guide

This guide will walk you through setting up your Kubernetes cluster with Gitea and CI/CD pipelines.

## Prerequisites

- Servers configured: frigg, baulder, oak
- SMB servers configured: maple (primary), willow (fallback)
- Environment variables loaded from 1Password via direnv

## Step 1: Clean Up Local Deployments

Remove any local deployments to start fresh:

```bash
# Uninstall local Helm releases
halvor helm uninstall pia-vpn -y

# Stop and remove local Docker containers
docker stop pia-vpn 2>/dev/null || true
docker rm pia-vpn 2>/dev/null || true

# Clean up local Kubernetes resources
kubectl delete deployment pia-vpn -n default 2>/dev/null || true
```

## Step 2: Setup SMB Mounts on Remote Servers

Setup SMB mounts on both frigg and baulder (the deployment nodes):

```bash
# Setup SMB mounts on frigg
halvor smb frigg

# Setup SMB mounts on baulder
halvor smb baulder
```

This will mount:
- `/mnt/smb/maple/backups` - Backups (primary)
- `/mnt/smb/maple/data` - Data (primary, shared)
- `/mnt/smb/maple/halvor` - Halvor container data (primary)
- `/mnt/smb/willow/backups` - Backups (fallback)
- `/mnt/smb/willow/data` - Data (fallback, shared)
- `/mnt/smb/willow/halvor` - Halvor container data (fallback)

## Step 3: Provision Servers for Cluster

Provision each server with the correct cluster role:

```bash
# Provision frigg as primary control plane
halvor provision frigg --cluster-role control-plane

# Provision baulder as HA control plane
halvor provision baulder --cluster-role control-plane

# Provision oak as tiebreaker
halvor provision oak --cluster-role tiebreaker
```

## Step 4: Deploy SMB Storage

Deploy the SMB storage chart to create PersistentVolumes:

```bash
halvor install smb-storage --helm
```

This creates:
- PersistentVolumes for all SMB shares
- StorageClass: `smb-storage`
- Shared PVC: `halvor-shared-data` (bound to halvor-data-primary PV)

## Step 5: Deploy Gitea

Deploy Gitea to host your code:

```bash
halvor install gitea --helm
```

Gitea will be available at the domain configured in your environment variables (GITEA_DOMAIN).

## Step 6: Configure Gitea Repository

1. Access Gitea at your configured domain
2. Create a new repository (e.g., `halvor`)
3. Get the repository URL

## Step 7: Add Gitea as Git Remote

Add Gitea as a remote for your local repository:

```bash
cd /Users/scottkey/code/halvor
git remote add gitea <gitea-repo-url>
git push gitea main
```

## Step 8: Setup CI/CD Pipeline

Create a Gitea Actions workflow (`.gitea/workflows/deploy-experimental.yml`):

```yaml
name: Deploy to Cluster (Experimental)

on:
  push:
    branches:
      - experimental
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Setup kubectl
        uses: azure/setup-kubectl@v3
        with:
          version: 'latest'
      
      - name: Setup Helm
        uses: azure/setup-helm@v3
        with:
          version: 'latest'
      
      - name: Configure kubectl
        run: |
          # Get kubeconfig from cluster
          # This should be stored as a secret in Gitea
          mkdir -p ~/.kube
          echo "${{ secrets.KUBECONFIG }}" > ~/.kube/config
      
      - name: Deploy Services
        run: |
          # Deploy all services using halvor CLI or Helm directly
          # This will be expanded based on your services
          helm upgrade --install smb-storage ./charts/smb-storage
          helm upgrade --install gitea ./charts/gitea
          # Add other services as needed
```

## Step 9: Setup Code Sync to GitHub

Create a workflow to sync code from Gitea to GitHub:

```yaml
name: Sync to GitHub

on:
  push:
    branches:
      - experimental
      - main
  workflow_dispatch:

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0
      
      - name: Configure Git
        run: |
          git config user.name "Gitea Bot"
          git config user.email "bot@gitea.local"
      
      - name: Add GitHub Remote
        run: |
          git remote add github https://github.com/scottdkey/halvor.git || true
          git remote set-url github https://${{ secrets.GITHUB_TOKEN }}@github.com/scottdkey/halvor.git
      
      - name: Push to GitHub
        run: |
          git push github ${{ github.ref_name }} --force
```

## Step 10: Setup Release Workflow

Create a workflow for official releases:

```yaml
name: Release to GitHub

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Create GitHub Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: false
```

## Next Steps

1. Complete the setup steps above
2. Test the CI/CD pipelines
3. Configure additional services as needed
4. Setup monitoring and logging

