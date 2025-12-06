FROM alpine:latest

# Install Privoxy and required tools
RUN apk add --no-cache \
    privoxy \
    curl \
    net-tools \
    bash

# Create necessary directories
RUN mkdir -p /etc/privoxy /var/log/privoxy /config

# Configure Privoxy
RUN echo "listen-address 0.0.0.0:8888" >> /etc/privoxy/config && \
    echo "forward / ." >> /etc/privoxy/config

# Copy entrypoint script
COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Expose HTTP proxy port
EXPOSE 8888/tcp

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
