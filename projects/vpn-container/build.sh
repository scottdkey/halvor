#!/bin/bash
# Build script for openvpn-container
# This script handles building and pushing the container image

set -e

# Default values
GITHUB_USER="${GITHUB_USER:-scottdkey}"
IMAGE_NAME="ghcr.io/${GITHUB_USER}/pia-vpn"
PUSH=false
RELEASE=false
NO_CACHE=false
CUSTOM_TAG=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --push)
            PUSH=true
            shift
            ;;
        --release)
            RELEASE=true
            shift
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --tag)
            CUSTOM_TAG="$2"
            shift 2
            ;;
        --github-user)
            GITHUB_USER="$2"
            IMAGE_NAME="ghcr.io/${GITHUB_USER}/pia-vpn"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--push] [--release] [--no-cache] [--tag TAG] [--github-user USER]"
            exit 1
            ;;
    esac
done

# Get git hash for versioning (if in git repo)
GIT_HASH="unknown"
if command -v git &> /dev/null && git rev-parse --git-dir > /dev/null 2>&1; then
    GIT_HASH=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
fi

# Determine tags
if [ -n "$CUSTOM_TAG" ]; then
    TAGS=("${IMAGE_NAME}:${CUSTOM_TAG}")
elif [ "$RELEASE" = true ]; then
    TAGS=("${IMAGE_NAME}:latest" "${IMAGE_NAME}:${GIT_HASH}")
else
    TAGS=("${IMAGE_NAME}:experimental" "${IMAGE_NAME}:${GIT_HASH}")
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building PIA VPN Container"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Image: ${IMAGE_NAME}"
echo "Tags: ${TAGS[*]}"
echo "Git hash: ${GIT_HASH}"
echo "Push: ${PUSH}"
echo "Release: ${RELEASE}"
echo ""

# Check if buildx is available
if ! docker buildx version &> /dev/null; then
    echo "⚠️  Docker buildx not available, using regular docker build"
    USE_BUILDX=false
else
    USE_BUILDX=true
fi

# Build the image
if [ "$USE_BUILDX" = true ] && [ "$PUSH" = true ]; then
    # Multi-platform build with buildx
    echo "Building multi-platform image (linux/amd64,linux/aarch64)..."
    
    # Ensure buildx builder exists
    if ! docker buildx inspect halvor-builder &> /dev/null; then
        echo "Creating Docker buildx builder 'halvor-builder'..."
        docker buildx create --name halvor-builder --use || true
    else
        docker buildx use halvor-builder || true
    fi
    
    # Build with buildx
    BUILD_ARGS=("buildx" "build" "--platform" "linux/amd64,linux/aarch64")
    
    if [ "$NO_CACHE" = true ]; then
        BUILD_ARGS+=("--no-cache")
    fi
    
    for tag in "${TAGS[@]}"; do
        BUILD_ARGS+=("-t" "$tag")
    done
    
    BUILD_ARGS+=("-f" "Dockerfile" ".")
    
    if [ "$PUSH" = true ]; then
        BUILD_ARGS+=("--push")
    fi
    
    docker "${BUILD_ARGS[@]}"
else
    # Single platform build
    echo "Building single-platform image..."
    
    BUILD_ARGS=("build")
    
    if [ "$NO_CACHE" = true ]; then
        BUILD_ARGS+=("--no-cache")
    fi
    
    for tag in "${TAGS[@]}"; do
        BUILD_ARGS+=("-t" "$tag")
    done
    
    BUILD_ARGS+=("-f" "Dockerfile" ".")
    
    docker "${BUILD_ARGS[@]}"
    
    if [ "$PUSH" = true ]; then
        echo ""
        echo "Pushing images..."
        for tag in "${TAGS[@]}"; do
            echo "Pushing ${tag}..."
            docker push "$tag"
        done
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✓ Build complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ "$PUSH" = true ]; then
    echo "Images pushed to: ${IMAGE_NAME}"
    echo ""
    echo "To use this image, update your Helm chart values:"
    echo "  image.repository: ${IMAGE_NAME}"
    if [ "$RELEASE" = true ]; then
        echo "  image.tag: latest"
    else
        echo "  image.tag: experimental"
    fi
else
    echo "Images built locally. To push, run with --push flag."
fi

