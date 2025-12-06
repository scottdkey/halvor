# GitHub Actions Workflows

This directory contains GitHub Actions workflows for building and releasing the homelab automation tools.

## Workflows

### `build.yml` - Continuous Integration

**Triggers:**
- Push to `main` branch (when relevant files change)
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs:**
1. **build-rust-cli**: Builds the `hal` CLI tool for multiple platforms:
   - Linux (x86_64, aarch64) - GNU and musl variants
   - macOS (x86_64, aarch64)
   - Uploads binaries as artifacts for use in other workflows

2. **build-docker-image**: Builds and pushes the VPN Docker image:
   - Image: `ghcr.io/{owner}/pia-vpn`
   - Tags: `latest`, branch name, SHA, and semantic version (if applicable)
   - Multi-platform: `linux/amd64` and `linux/arm64`
   - Only pushes on non-PR events

3. **build-summary**: Provides a summary of all build jobs

### `release.yml` - Release Workflow

**Triggers:**
- GitHub release creation
- Manual workflow dispatch (with version input)

**Jobs:**
1. **release-rust-cli**: Builds release binaries for all platforms and creates tarballs
2. **release-docker-image**: Builds and pushes Docker image with version tag

## Using the Artifacts

### Download Rust CLI Binary

After a workflow run, you can download the `hal` binary from the workflow artifacts:

1. Go to the Actions tab in GitHub
2. Select the workflow run
3. Download the artifact for your platform (e.g., `hal-x86_64-unknown-linux-gnu`)
4. Extract and use the binary

### Using the Docker Image

The Docker image is automatically pushed to GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/scottdkey/pia-vpn:latest

# Or use a specific version
docker pull ghcr.io/scottdkey/pia-vpn:v1.0.0
```

Make sure you're authenticated with GitHub Container Registry:

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u scottdkey --password-stdin
```

## Permissions

The workflows require the following GitHub permissions:
- `contents: read` - To checkout code
- `packages: write` - To push Docker images to GHCR
- `actions: write` - To upload artifacts

These are automatically granted via `GITHUB_TOKEN` in GitHub Actions.

## Local Testing

To test the builds locally:

```bash
# Build Rust CLI
cargo build --release

# Build Docker image
cd openvpn-container
docker build -t ghcr.io/scottdkey/pia-vpn:latest .
```
