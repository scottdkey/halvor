# GitHub Actions Workflows

This document describes the CI/CD workflows for the halvor project.

## Workflows

### `build.yml` - Continuous Integration

**Triggers:**

- Push to `main` branch (when relevant files change)
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs:**

1. **build-rust-cli-linux**: Builds the `halvor` CLI tool for Linux platforms:
   - Linux (x86_64, aarch64) - GNU and musl variants
   - Uploads binaries as artifacts for use in other workflows

2. **build-rust-cli-macos**: Builds the `halvor` CLI tool for macOS platforms:
   - macOS (x86_64, aarch64)
   - Runs on `macos-14` runners

3. **build-pia-vpn**: Builds and pushes the PIA VPN Docker image:
   - Image: `ghcr.io/{owner}/pia-vpn`
   - Tags: `latest`, version tag, and experimental (for main branch pushes)
   - Multi-platform: `linux/amd64` and `linux/arm64`
   - Triggers on changes to `vpn-container/**` or manual dispatch

4. **build-summary**: Provides a summary of all build jobs

### `release.yml` - Release Workflow

**Triggers:**

- GitHub release creation
- Manual workflow dispatch (with version input)

**Jobs:**

1. **release-rust-cli-linux**: Builds release binaries for Linux platforms and creates tarballs
2. **release-rust-cli-macos**: Builds release binaries for macOS platforms and creates tarballs
3. **release-docker-image**: Builds and pushes Docker image with version tag

## Using the Artifacts

### Download Rust CLI Binary

After a workflow run, you can download the `halvor` binary from the workflow artifacts:

1. Go to the Actions tab in GitHub
2. Select the workflow run
3. Download the artifact for your platform (e.g., `halvor-x86_64-unknown-linux-gnu`)
4. Extract and use the binary

### Using the Docker Image

The Docker image is automatically pushed to GitHub Container Registry:

```bash
# Pull the latest image
docker pull ghcr.io/<your-username>/pia-vpn:latest

# Or use a specific version
docker pull ghcr.io/<your-username>/pia-vpn:v1.0.0

# Or use experimental (continuously updated from main branch)
docker pull ghcr.io/<your-username>/pia-vpn:experimental
```

Make sure you're authenticated with GitHub Container Registry:

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u <your-username> --password-stdin
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
cd vpn-container
docker build -t ghcr.io/<your-username>/vpn:latest .
```

## Troubleshooting

If you encounter issues with the workflows, see:

- [GHCR Setup Guide](../.github/workflows/GHCR_SETUP.md) - For Docker image push issues
- [Workflow Testing Guide](../.github/workflows/WORKFLOW_TESTING.md) - For local workflow testing
