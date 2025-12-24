# OpenVPN Container (PIA VPN)

This container provides Private Internet Access (PIA) VPN connectivity with an HTTP proxy for other containers.

## Building

The container includes its own build script that handles building and pushing:

```bash
# Build locally (single platform)
./build.sh

# Build and push to registry (multi-platform)
./build.sh --push

# Build with release tag (latest instead of experimental)
./build.sh --push --release

# Build without cache
./build.sh --no-cache

# Custom GitHub user/organization
GITHUB_USER=myorg ./build.sh --push
```

## Using with halvor

The container can be built using the centralized docker build system:

```bash
# Build locally
halvor build pia-vpn

# Build and push
halvor build pia-vpn --push

# Build with release tag
halvor build pia-vpn --push --release
```

Or using the VPN command:

```bash
# Build and push
halvor vpn build --push

# Build with release tag
halvor vpn build --push --release
```

The build script automatically:
- Detects git hash for versioning
- Builds multi-platform images (linux/amd64, linux/aarch64) when pushing
- Uses Docker buildx for multi-platform builds
- Handles authentication and registry push

## Deployment

Deploy to Kubernetes using Helm:

```bash
halvor install pia-vpn --helm
```

Or install directly:

```bash
halvor helm install pia-vpn
```

