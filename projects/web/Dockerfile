# Multi-stage Dockerfile for Halvor Web Application
# Supports both development and production builds

# ============================================================================
# Stage 1: Build Rust Server
# ============================================================================
FROM rust:1.85-slim AS rust-builder

WORKDIR /build

# Install build dependencies (including musl for Alpine production)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

# Install musl targets for static linking (needed for Alpine)
# Install both ARM and x86_64 targets for cross-platform support
RUN rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl

# Copy workspace Cargo files first (for better caching)
COPY Cargo.toml Cargo.lock ./
# Copy all crates (halvor-cli and its dependencies)
COPY crates ./crates

# Build Rust server (release or debug based on BUILD_TYPE)
# Build musl binary for current platform (TARGETARCH set by Docker buildx)
# Workaround for FFI symbol conflicts: use linker flag to allow multiple definitions
ARG BUILD_TYPE=release
ARG TARGETARCH
ARG TARGETPLATFORM
ENV RUSTFLAGS="-C link-arg=-Wl"
ENV PKG_CONFIG_ALL_STATIC=1

# Determine musl target based on TARGETARCH (set by Docker buildx)
# TARGETARCH: amd64, arm64, etc.
RUN ARCH=${TARGETARCH:-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')} && \
    echo "Building for architecture: $ARCH (platform: ${TARGETPLATFORM:-unknown})" && \
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then \
        MUSL_TARGET="aarch64-unknown-linux-musl"; \
    else \
        MUSL_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    echo "Using musl target: ${MUSL_TARGET}" && \
    if [ "$BUILD_TYPE" = "release" ]; then \
        echo "Building musl binary for production (${MUSL_TARGET})..." && \
        cargo build --release -p halvor-cli --target ${MUSL_TARGET} && \
        ls -lh /build/target/${MUSL_TARGET}/release/halvor && \
        echo "Building glibc binary for development..." && \
        cargo build --release -p halvor-cli && \
        ls -lh /build/target/release/halvor; \
    else \
        # Build glibc binary for development
        cargo build -p halvor-cli && \
        ls -lh /build/target/debug/halvor; \
    fi

# ============================================================================
# Stage 2: Build Svelte Frontend
# ============================================================================
FROM node:24-alpine AS svelte-builder

WORKDIR /app

# Copy package files from halvor-web directory
COPY projects/web/package.json projects/web/package-lock.json* ./
RUN npm ci

# Copy Svelte source from halvor-web directory
COPY projects/web/ ./

# Build the Svelte app (production or development based on BUILD_TYPE)
ARG BUILD_TYPE=release
RUN if [ "$BUILD_TYPE" = "release" ]; then \
        npm run build -- --mode production || npm run build; \
    else \
        npm run build; \
    fi && \
    # Ensure build directory exists (adapter-auto may not create static build)
    mkdir -p /app/build || true

# ============================================================================
# Stage 3: Development Runtime (with hot reload)
# ============================================================================
# Use Debian-based Node image to match rust-builder (glibc compatibility)
FROM node:24-bookworm-slim AS development

WORKDIR /app

# Install development dependencies (Rust toolchain for potential rebuilds + curl for health checks)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust binary from builder (glibc binary for Debian-based dev image)
ARG BUILD_TYPE=release
COPY --from=rust-builder /build/target/${BUILD_TYPE}/halvor /usr/local/bin/halvor
RUN chmod +x /usr/local/bin/halvor && \
    ls -la /usr/local/bin/halvor && \
    /usr/local/bin/halvor --version || echo "Warning: Binary verification failed"

# Copy Svelte source (for hot reload)
COPY projects/web/package.json projects/web/package-lock.json* ./
RUN npm install

COPY projects/web/ ./

# Expose ports (dev server + web server)
EXPOSE 13001 13000

# Set environment variables
ENV RUST_LOG=info
ENV HALVOR_WEB_DIR=/app

# Health check script for development
RUN echo '#!/bin/sh\n\
curl -f http://localhost:13000/api/health || exit 1' > /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD ["/app/healthcheck.sh"]

# Start dev server with watch mode
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

# ============================================================================
# Stage 4: Production Runtime
# ============================================================================
FROM alpine:3.20 AS production

WORKDIR /app

# Install runtime dependencies (including curl for health checks)
RUN apk add --no-cache \
    ca-certificates \
    openssl \
    curl \
    libgcc

# Health check script
RUN echo '#!/bin/sh\n\
curl -f http://localhost:13000/api/health || exit 1' > /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

# Copy Rust binary from builder (static musl binary for Alpine)
# Use TARGETARCH to determine which binary to copy
ARG BUILD_TYPE=release
ARG TARGETARCH
# Copy the entire target directory so we can select the right binary
COPY --from=rust-builder /build/target /build/target
RUN ARCH=${TARGETARCH:-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')} && \
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then \
        MUSL_TARGET="aarch64-unknown-linux-musl"; \
    else \
        MUSL_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    echo "Copying binary for ${MUSL_TARGET}..." && \
    cp "/build/target/${MUSL_TARGET}/${BUILD_TYPE}/halvor" /usr/local/bin/halvor && \
    chmod +x /usr/local/bin/halvor && \
    /usr/local/bin/halvor --version || echo "Binary verification failed"

# Copy Svelte build from builder
COPY --from=svelte-builder /app/build ./halvor-web/build

# Create directory for SQLite database (will be mounted from host)
RUN mkdir -p /app/data

# Expose web server port (13000)
EXPOSE 13000

# Set environment variables
ENV RUST_LOG=info
ENV HALVOR_WEB_DIR=/app/halvor-web
ENV HALVOR_DB_DIR=/app/data

# Health check script
RUN echo '#!/bin/sh\n\
curl -f http://localhost:13000/api/health || exit 1' > /app/healthcheck.sh && \
    chmod +x /app/healthcheck.sh

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD ["/app/healthcheck.sh"]

# Run the server (agent on 13001, web on 13000)
# Web UI: http://localhost:13000
# Agent API (for CLI): port 13001
CMD ["halvor", "agent", "start", "--port", "13001", "--web-port", "13000"]
