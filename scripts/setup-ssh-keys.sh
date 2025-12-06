#!/bin/bash

# Setup SSH keys on remote hosts
# Uses password authentication initially, then sets up key-based auth
# After keys are set up, password authentication is no longer needed

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <hostname> [username]"
    echo ""
    echo "Sets up SSH key authentication on a remote host."
    echo "You will be prompted for the password once to copy your SSH key."
    echo "After this, password authentication is no longer needed."
    echo ""
    echo "The hostname should match an entry in your ~/.ssh/config"
    echo "or be specified as user@hostname"
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

HOST_SPEC="$1"
USERNAME="$2"

# Parse hostname and username
if [[ "$HOST_SPEC" == *"@"* ]]; then
    USERNAME="${HOST_SPEC%%@*}"
    HOSTNAME="${HOST_SPEC#*@}"
else
    HOSTNAME="$HOST_SPEC"
    if [ -z "$USERNAME" ]; then
        # Try to get username from SSH config
        if [ -f "$HOME/.ssh/config" ]; then
            USERNAME=$(grep -A 10 "^Host $HOSTNAME$" "$HOME/.ssh/config" | grep -i "User" | awk '{print $2}' | head -1)
        fi
        
        if [ -z "$USERNAME" ]; then
            USERNAME=$(whoami)
        fi
    fi
fi

echo -e "${GREEN}Setting up SSH keys for ${USERNAME}@${HOSTNAME}${NC}"
echo ""

# Check if SSH key exists
SSH_KEY=""
if [ -f "$HOME/.ssh/id_ed25519.pub" ]; then
    SSH_KEY="$HOME/.ssh/id_ed25519.pub"
elif [ -f "$HOME/.ssh/id_rsa.pub" ]; then
    SSH_KEY="$HOME/.ssh/id_rsa.pub"
elif [ -f "$HOME/.ssh/id_ecdsa.pub" ]; then
    SSH_KEY="$HOME/.ssh/id_ecdsa.pub"
else
    echo -e "${RED}Error: No SSH public key found${NC}"
    echo "Generate one with: ssh-keygen -t ed25519 -C \"your_email@example.com\""
    exit 1
fi

echo -e "${GREEN}Using SSH key: ${SSH_KEY}${NC}"
echo ""

# Test if key is already installed
echo "Testing if SSH key is already installed..."
if ssh -o ConnectTimeout=5 -o BatchMode=yes -o PreferredAuthentications=publickey -o PasswordAuthentication=no "$USERNAME@$HOSTNAME" "echo 'Key auth works'" 2>/dev/null; then
    echo -e "${GREEN}✓ SSH key is already installed and working${NC}"
    echo "You can connect without a password using: ssh $HOSTNAME"
    exit 0
fi

echo -e "${YELLOW}SSH key not found or not working. Setting up...${NC}"
echo ""
echo "You will be prompted for the password for ${USERNAME}@${HOSTNAME}"
echo "This is the only time you'll need to enter the password."
echo ""

# Copy SSH key using ssh-copy-id
if command -v ssh-copy-id &> /dev/null; then
    ssh-copy-id -o StrictHostKeyChecking=no "$USERNAME@$HOSTNAME"
else
    # Manual method if ssh-copy-id is not available
    PUBKEY=$(cat "$SSH_KEY")
    ssh -o StrictHostKeyChecking=no "$USERNAME@$HOSTNAME" \
        "mkdir -p ~/.ssh && chmod 700 ~/.ssh && \
         echo '$PUBKEY' >> ~/.ssh/authorized_keys && \
         chmod 600 ~/.ssh/authorized_keys"
fi

echo ""
echo -e "${GREEN}✓ SSH key installed successfully${NC}"
echo ""

# Test the connection
echo "Testing key-based authentication..."
if ssh -o ConnectTimeout=5 -o BatchMode=yes -o PreferredAuthentications=publickey -o PasswordAuthentication=no "$USERNAME@$HOSTNAME" "echo 'Connection successful'" 2>/dev/null; then
    echo -e "${GREEN}✓ Key-based authentication is working!${NC}"
    echo ""
    echo "You can now connect without a password:"
    echo "  ssh $HOSTNAME"
    echo ""
    echo -e "${YELLOW}Note: Password authentication is still enabled on the remote host.${NC}"
    echo "To disable it (recommended for security), run on the remote host:"
    echo "  sudo sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config"
    echo "  sudo sed -i 's/PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config"
    echo "  sudo systemctl restart sshd"
else
    echo -e "${RED}⚠ Warning: Key-based authentication test failed${NC}"
    echo "You may need to check the remote host's SSH configuration"
fi
