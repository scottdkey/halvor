#!/bin/bash
# Test the exact shell command from the Makefile

OS=$(uname -s);
if [ "$OS" = "Darwin" ]; then
	if launchctl list com.halvor.agent 2>/dev/null | grep -q .; then
		echo "Stopping halvor agent service...";
		launchctl stop com.halvor.agent 2>/dev/null || true;
		launchctl unload ~/Library/LaunchAgents/com.halvor.agent.plist 2>/dev/null || true;
		sleep 1;
	fi;
	pkill -9 -f "halvor agent" 2>/dev/null || true;
fi;
if [ "$OS" = "Linux" ]; then
	# Stop user service if running
	if systemctl --user is-active halvor-agent.service >/dev/null 2>&1; then
		echo "Stopping halvor agent service (user service)...";
		systemctl --user stop halvor-agent.service 2>/dev/null || true;
		sleep 1;
	fi;
	# Stop system service if running (will migrate to user service)
	if systemctl is-active halvor-agent.service >/dev/null 2>&1; then
		echo "Stopping halvor agent service (system service, will migrate to user service)...";
		sudo systemctl stop halvor-agent.service 2>/dev/null || true;
		sleep 1;
	fi;
fi

OS=$(uname -s);
if [ "$OS" = "Darwin" ]; then
	if [ -f ~/Library/LaunchAgents/com.halvor.agent.plist ]; then
		echo "Restarting halvor agent service...";
		launchctl load -w ~/Library/LaunchAgents/com.halvor.agent.plist 2>/dev/null || true;
		launchctl start com.halvor.agent 2>/dev/null || true;
		echo "Agent service restarted";
	fi;
fi;
if [ "$OS" = "Linux" ]; then
	if [ -f ~/.config/systemd/user/halvor-agent.service ]; then
		echo "Restarting halvor agent service (user service)...";
		systemctl --user daemon-reload 2>/dev/null || true;
		systemctl --user restart halvor-agent.service 2>/dev/null || true;
		echo "✓ Agent service restarted";
	elif [ -f /etc/systemd/system/halvor-agent.service ]; then
		echo "Migrating halvor agent service from system to user service...";
		sudo systemctl stop halvor-agent.service 2>/dev/null || true;
		sudo systemctl disable halvor-agent.service 2>/dev/null || true;
		chmod +x scripts/setup-agent-service.sh 2>/dev/null || true;
		bash scripts/setup-agent-service.sh;
		echo "✓ Service migrated to user service";
	else
		echo "Setting up halvor agent service as user service (no sudo required)...";
		chmod +x scripts/setup-agent-service.sh 2>/dev/null || true;
		bash scripts/setup-agent-service.sh;
	fi;
fi

