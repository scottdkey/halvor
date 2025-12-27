#!/bin/bash
# Setup halvor agent service as a user service (no sudo required)
# This script is called from the Makefile to avoid complex nested if statements
# Usage: setup-agent-service.sh [migrate]
#   If "migrate" is passed, it will stop/disable the system service first

# Don't use set -e when called from Makefile, as we want to handle errors gracefully
MIGRATE_MODE="$1"

HALVOR_PATH=$(which halvor || echo ~/.cargo/bin/halvor)
HOME_DIR="$HOME"
CONFIG_DIR="$HOME_DIR/.config/halvor"
PID_FILE="$CONFIG_DIR/halvor-agent.pid"
WEB_DIR="${HALVOR_WEB_DIR:-/opt/halvor/projects/web}"
WEB_PORT_ARG=""
if [ -n "$HALVOR_WEB_DIR" ]; then
    WEB_PORT_ARG=" --web-port 13000"
fi

# Migrate from system service if requested
if [ "$MIGRATE_MODE" = "migrate" ]; then
    echo "Stopping and disabling system service..."
    sudo systemctl stop halvor-agent.service 2>/dev/null || true
    sudo systemctl disable halvor-agent.service 2>/dev/null || true
fi

mkdir -p ~/.config/systemd/user
mkdir -p "$CONFIG_DIR"

# Create user service file
cat > ~/.config/systemd/user/halvor-agent.service <<EOF
[Unit]
Description=Halvor Agent - Secure cluster management service
After=network.target tailscale.service
Wants=network.target

[Service]
Type=forking
ExecStart=$HALVOR_PATH agent start --port 13500$WEB_PORT_ARG --daemon
PIDFile=$PID_FILE
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

Environment="HOME=$HOME_DIR"
Environment="HALVOR_DB_DIR=$CONFIG_DIR"
Environment="HALVOR_WEB_DIR=$WEB_DIR"

NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable halvor-agent.service 2>/dev/null || true
systemctl --user start halvor-agent.service 2>/dev/null || true

echo "✓ Service set up as user service"
echo "✓ Agent service started"

