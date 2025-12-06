FROM alpine:latest

# Update package index and install OpenVPN, Privoxy and required tools
RUN apk update && apk add --no-cache \
    openvpn \
    privoxy \
    curl \
    iptables \
    bash \
    net-tools \
    unzip \
    wget \
    ca-certificates

# Create necessary directories
RUN mkdir -p /etc/openvpn /var/log/openvpn /etc/privoxy /var/log/privoxy /config

# Configure Privoxy
RUN echo "listen-address 0.0.0.0:8888" >> /etc/privoxy/config && \
    echo "forward / ." >> /etc/privoxy/config

# Copy entrypoint script
COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose OpenVPN port and Privoxy HTTP proxy port
EXPOSE 1194/udp 8888/tcp

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
