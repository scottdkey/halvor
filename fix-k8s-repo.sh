#!/bin/bash
# Fix Kubernetes repository GPG key signature warning
# Re-adds the repository using the official method to get a fresh GPG key

set -e

echo "Removing old Kubernetes repository configuration..."
sudo rm -f /etc/apt/sources.list.d/kubernetes.sources
sudo rm -f /etc/apt/sources.list.d/kubernetes.list
sudo rm -f /etc/apt/keyrings/kubernetes-apt-keyring.gpg

echo ""
echo "Adding Kubernetes repository using official method..."
# Create directory for keyring if it doesn't exist
sudo mkdir -p /etc/apt/keyrings

# Download and add the GPG key using the official method
curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg

# Add the repository using the new format
echo "Types: deb
URIs: https://pkgs.k8s.io/core:/stable:/v1.28/deb/
Suites: /
Components: 
Signed-By: /etc/apt/keyrings/kubernetes-apt-keyring.gpg" | sudo tee /etc/apt/sources.list.d/kubernetes.sources > /dev/null

echo ""
echo "Verifying the configuration..."
cat /etc/apt/sources.list.d/kubernetes.sources

echo ""
echo "Updating apt package lists..."
sudo apt-get update 2>&1 | grep -v "^W:" || true

echo ""
echo "Checking for warnings..."
WARNINGS=$(sudo apt-get update 2>&1 | grep -c "Policy will reject signature" || true)

if [ "$WARNINGS" -gt 0 ]; then
    echo ""
    echo "⚠️  Warning detected: GPG key uses SHA-1 signatures (deprecated in 2026)"
    echo ""
    echo "Options:"
    echo "  1. Suppress the warning (temporary workaround)"
    echo "  2. Wait for Kubernetes team to update keys (recommended)"
    echo ""
    printf "Suppress warning? (y/N): "
    read REPLY
    if [ "$REPLY" = "y" ] || [ "$REPLY" = "Y" ]; then
        echo "Note: This warning cannot be fully suppressed without compromising security."
        echo "The warning is informational - the repository will continue to work."
        echo "Kubernetes team will update their GPG keys before 2026-02-01."
        echo ""
        echo "To reduce noise, you can filter the warning:"
        echo "  sudo apt-get update 2>&1 | grep -v 'Policy will reject signature'"
    else
        echo "Keeping warning visible - Kubernetes team will update keys before 2026-02-01"
        echo "Repository will continue to work normally until then"
    fi
else
    echo "✓ Kubernetes repository updated successfully - no warnings!"
fi

