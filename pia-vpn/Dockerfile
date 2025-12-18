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
    && rm -rf /var/cache/apk/*

# Create necessary directories
RUN mkdir -p /etc/openvpn /var/log/openvpn /etc/privoxy /var/log/privoxy /config

# Configure Privoxy
RUN echo "confdir /etc/privoxy" >> /etc/privoxy/config && \
    echo "listen-address 0.0.0.0:8888" >> /etc/privoxy/config && \
    echo "forward / ." >> /etc/privoxy/config && \
    echo "logfile /var/log/privoxy/logfile" >> /etc/privoxy/config && \
    echo "logdir /var/log/privoxy" >> /etc/privoxy/config

# Copy scripts
COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
COPY scripts/tail-logs.sh /usr/local/bin/tail-logs.sh
COPY scripts/check-proxy.sh /usr/local/bin/check-proxy.sh
COPY scripts/test-vpn.sh /usr/local/bin/test-vpn.sh
RUN chmod +x /usr/local/bin/entrypoint.sh /usr/local/bin/tail-logs.sh /usr/local/bin/check-proxy.sh /usr/local/bin/test-vpn.sh

# Expose OpenVPN port and Privoxy HTTP proxy port
EXPOSE 1194/udp 8888/tcp

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
