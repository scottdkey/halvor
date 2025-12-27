#!/bin/bash
# Extract and test the shell command from the Makefile

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

