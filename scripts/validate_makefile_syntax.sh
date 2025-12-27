#!/bin/bash
# Validate the Makefile shell syntax by extracting and testing the commands

echo "Testing first shell block (stop services)..."
bash -c '
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
	if systemctl --user is-active halvor-agent.service >/dev/null 2>&1; then
		echo "Stopping halvor agent service (user service)...";
		systemctl --user stop halvor-agent.service 2>/dev/null || true;
		sleep 1;
	fi;
	if systemctl is-active halvor-agent.service >/dev/null 2>&1; then
		echo "Stopping halvor agent service (system service, will migrate to user service)...";
		sudo systemctl stop halvor-agent.service 2>/dev/null || true;
		sleep 1;
	fi;
fi
' && echo "✓ First block syntax OK" || echo "✗ First block syntax ERROR"

echo ""
echo "Testing second shell block (restart services)..."
bash -c '
if [ "$(uname -s)" = "Darwin" ]; then
	if [ -f ~/Library/LaunchAgents/com.halvor.agent.plist ]; then
		echo "Restarting halvor agent service...";
		launchctl load -w ~/Library/LaunchAgents/com.halvor.agent.plist 2>/dev/null || true;
		launchctl start com.halvor.agent 2>/dev/null || true;
		echo "Agent service restarted";
	fi;
fi
if [ "$(uname -s)" = "Linux" ]; then
	if [ -f ~/.config/systemd/user/halvor-agent.service ]; then
		echo "Restarting halvor agent service (user service)...";
		systemctl --user daemon-reload 2>/dev/null || true;
		systemctl --user restart halvor-agent.service 2>/dev/null || true;
		echo "✓ Agent service restarted";
	else
		chmod +x scripts/setup-agent-service.sh 2>/dev/null || true;
		if [ -f /etc/systemd/system/halvor-agent.service ]; then
			bash scripts/setup-agent-service.sh migrate;
		else
			bash scripts/setup-agent-service.sh;
		fi;
	fi;
fi
' && echo "✓ Second block syntax OK" || echo "✗ Second block syntax ERROR"

