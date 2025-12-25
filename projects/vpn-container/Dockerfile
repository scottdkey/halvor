# Multi-stage Dockerfile for PIA VPN Container
# Builds Rust entrypoint and includes OpenVPN/Privoxy

# ============================================================================
# Stage 1: Build Rust Entrypoint
# ============================================================================
FROM rust:1.85-slim AS rust-builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

# Install musl targets for static linking (needed for Alpine)
ARG TARGETARCH
RUN ARCH=${TARGETARCH:-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')} && \
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then \
        rustup target add aarch64-unknown-linux-musl; \
        MUSL_TARGET="aarch64-unknown-linux-musl"; \
    else \
        rustup target add x86_64-unknown-linux-musl; \
        MUSL_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    echo "Using musl target: ${MUSL_TARGET}"

# Copy Cargo files
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build Rust binary with musl for static linking
RUN ARCH=${TARGETARCH:-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')} && \
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then \
        MUSL_TARGET="aarch64-unknown-linux-musl"; \
    else \
        MUSL_TARGET="x86_64-unknown-linux-musl"; \
    fi && \
    cargo build --release --bin entrypoint --target ${MUSL_TARGET} && \
    ls -lh /build/target/${MUSL_TARGET}/release/entrypoint

# ============================================================================
# Stage 2: Runtime (Alpine with OpenVPN and Privoxy)
# ============================================================================
FROM alpine:latest

# Install OpenVPN, Privoxy and required tools
RUN apk update && \
    apk add --no-cache \
    openvpn \
    privoxy \
    curl \
    iptables \
    bash \
    net-tools \
    unzip \
    wget \
    dos2unix \
    sed \
    ca-certificates \
    procps \
    && rm -rf /var/cache/apk/*

# Create necessary directories
RUN mkdir -p /etc/openvpn /var/log/openvpn /etc/privoxy /var/log/privoxy /config

# Configure Privoxy base config (port set at runtime via entrypoint)
RUN echo "confdir /etc/privoxy" > /etc/privoxy/config && \
    echo "forward / ." >> /etc/privoxy/config && \
    echo "logfile /var/log/privoxy/logfile" >> /etc/privoxy/config && \
    echo "logdir /var/log/privoxy" >> /etc/privoxy/config

# Default proxy port (can be overridden via PROXY_PORT env var)
ENV PROXY_PORT=8888

# Copy Rust entrypoint binary from builder
COPY --from=rust-builder /build/target/*/release/entrypoint /usr/local/bin/entrypoint
RUN chmod +x /usr/local/bin/entrypoint

# Note: Helper scripts (tail-logs.sh, check-proxy.sh, test-vpn.sh) are optional
# The Rust entrypoint handles all functionality. If you need the scripts,
# copy them from compose/pia-vpn/scripts/ when building with repo root as context

# Expose OpenVPN port and Privoxy HTTP proxy port
EXPOSE 1194/udp 8888/tcp

ENTRYPOINT ["/usr/local/bin/entrypoint"]

