#!/bin/bash
set -e

echo "Starting Privoxy HTTP proxy..."
echo "Privoxy will listen on 0.0.0.0:8888"
echo "Traffic will route through VPN's TUN interface when VPN is connected"
exec privoxy --no-daemon /etc/privoxy/config
