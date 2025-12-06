# GitHub Workflows Testing Guide

## Pre-Push Validation

Before pushing to GitHub, run the validation script:

```bash
./.github/workflows/test-workflows.sh
```

This checks:
- ✅ All workflow files exist
- ✅ All referenced scripts exist
- ✅ Docker container directory exists (if building images)
- ✅ Rust project files exist
- ✅ YAML syntax (if Python yaml module available)

## Testing Workflows

### Option 1: Push to GitHub (Recommended)

The workflows will automatically run on:
- Push to `main` branch
- Pull requests to `main`
- Manual workflow dispatch

**To test manually:**
1. Push your changes to GitHub
2. Go to Actions tab in your repository
3. Select the workflow
4. Click "Run workflow" → Choose branch → Run

### Option 2: Use `act` (Local Testing)

Install [act](https://github.com/nektos/act) to run workflows locally:

```bash
# Install act (macOS)
brew install act

# Run the build workflow
act push

# Run specific job
act -j build-rust-cli
```

**Note:** `act` has limitations:
- Docker builds may not work exactly as on GitHub
- Some actions may not be fully supported
- Requires Docker to be running

## Current Status

### ✅ Working
- Rust CLI builds for all platforms
- Install scripts are uploaded as artifacts
- Scripts reference GitHub-hosted files correctly

### ⚠️ Known Issues
- **Docker Image Build**: Requires `openvpn-container/` directory
  - If directory doesn't exist, Docker build is skipped
  - Workflow will still succeed (Rust CLI will build)
  - To enable Docker builds, ensure `openvpn-container/Dockerfile` exists

### 📝 Workflow Behavior

**Build Workflow (`build.yml`):**
- Always builds Rust CLI for all platforms
- Builds Docker image only if `openvpn-container/` exists
- Uploads install scripts as artifacts
- Only pushes Docker images on non-PR events

**Release Workflow (`release.yml`):**
- Builds release binaries for all platforms
- Creates tarballs with version tags
- Uploads to GitHub Releases
- Builds and pushes Docker image with version tag
- Skips Docker build if `openvpn-container/` doesn't exist

## Troubleshooting

### Docker Build Fails
- Check that `openvpn-container/Dockerfile` exists
- Verify Dockerfile syntax is correct
- Check GitHub Container Registry permissions

### Rust Build Fails
- Verify `Cargo.toml` is valid
- Check that all dependencies are available
- Review build logs for specific errors

### Scripts Not Uploaded
- Verify scripts exist in `scripts/` directory
- Check that paths in workflow match actual file locations
- Ensure scripts have execute permissions

## Next Steps After Push

1. **Check Actions Tab**: Verify workflows run successfully
2. **Download Artifacts**: Test the built binaries
3. **Verify Docker Image**: Check GHCR for pushed images
4. **Test Install Scripts**: Run install scripts from artifacts
