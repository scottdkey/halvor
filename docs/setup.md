# Setup Guide

## Configure SSH Hosts

First, set up your SSH hosts from `.env` configuration:

```bash
./scripts/setup-ssh-hosts.sh
```

This reads SSH host configurations from your `.env` file and adds them to `~/.ssh/config`. Add
entries like:

```bash
SSH_MAPLE_HOST="10.10.10.130"
SSH_MAPLE_USER="skey"
SSH_MAPLE_PORT="22"

SSH_BELLEROPHON_HOST="10.10.10.14"
SSH_BELLEROPHON_USER="username"
```

## Setup SSH Keys

After configuring hosts, set up SSH key authentication (one-time password required):

```bash
./scripts/setup-ssh-keys.sh maple
```

This will:

- Copy your SSH public key to the remote host
- Prompt for password once (only time needed)
- Enable passwordless SSH connections

## Install Tailscale

Install Tailscale on your system (supports macOS, Linux, and Windows):

```bash
hal tailscale install
```

This will:

- Detect your operating system
- Use the appropriate package manager (Homebrew on macOS, apt/yum/dnf on Linux)
- Provide instructions for starting Tailscale

## Provision a Remote Host

Provision a remote host with Docker, Tailscale, and Portainer:

```bash
hal provision bellerophon
```

This will:

- Connect to the host via SSH (prompts for username and password)
- Install Docker if not already installed
- Install Tailscale if not already installed
- Install Portainer Agent (or Portainer CE with `--portainer-host` flag)
- Handle all sudo prompts interactively

**Install Portainer CE instead of Agent:**

```bash
hal provision bellerophon --portainer-host
```

This installs the full Portainer CE with web UI instead of just the agent.
