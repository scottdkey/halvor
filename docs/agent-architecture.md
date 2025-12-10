# Halvor Agent Architecture

## Overview

Halvor will support a mesh network architecture where each host runs a halvor agent daemon that:

- Automatically discovers other halvor agents on the network
- Syncs configuration data bidirectionally
- Provides secure remote command execution
- Maintains host information (IPs, Tailscale addresses, etc.)

## Architecture Components

### 1. Halvor Agent Daemon (`halvor agent`)

A background service that runs on each host:

- Listens on a configurable port (default: 23500)
- Maintains secure connections to other agents
- Handles remote command execution requests
- Syncs database/config state with peers
- Auto-discovers hosts via Tailscale/local network

### 2. Secure Communication

- **TLS/mTLS**: Each agent generates a certificate on first run
- **Shared Secret**: Optional shared secret for additional authentication
- **Token-based**: Short-lived tokens for command execution
- **Fallback to SSH**: If agent unavailable, fall back to existing SSH mechanism

### 3. Host Discovery

- **Tailscale Integration**: Query Tailscale API to discover other halvor agents
- **Local Network Scan**: Scan local network for halvor agents
- **Manual Registration**: Allow manual host registration
- **Auto-sync**: Automatically sync host info (IP, Tailscale address, hostname)

### 4. Config Sync Mesh

- **Bidirectional Sync**: Configs sync between all connected agents
- **Conflict Resolution**: Last-write-wins or manual resolution
- **Encrypted Data**: Encrypted env data syncs securely
- **Database Replication**: SQLite database syncs between hosts

### 5. Command Execution API

- **RPC-style API**: JSON-RPC or gRPC for command execution
- **Streaming Output**: Support for streaming command output
- **File Operations**: Secure file transfer and operations
- **Permission Model**: Role-based access control

## Implementation Plan

### Phase 1: Core Agent Infrastructure

1. Create `halvor agent` command (start/stop/status)
2. Basic HTTP/HTTPS server for agent communication
3. Simple authentication mechanism
4. Host discovery via Tailscale

### Phase 2: Secure Communication

1. TLS certificate generation and management
2. mTLS for mutual authentication
3. Token-based session management

### Phase 3: Config Sync

1. Database sync mechanism
2. Config file sync
3. Conflict resolution

### Phase 4: Command Execution

1. Remote command execution API
2. Update Executor to use agent API
3. SSH fallback mechanism

### Phase 5: Mesh Networking

1. Automatic peer discovery
2. Mesh topology management
3. Health checks and reconnection

## Security Considerations

- All communication encrypted (TLS)
- Mutual authentication (mTLS)
- Token expiration and rotation
- Rate limiting on API endpoints
- Audit logging
- Optional shared secret for additional security

## Benefits

1. **Better Security**: TLS/mTLS instead of plain SSH
2. **Automatic Discovery**: No manual host configuration needed
3. **Config Sync**: Changes propagate automatically
4. **Fault Tolerance**: Mesh network provides redundancy
5. **Performance**: Direct connections, no SSH overhead
6. **Scalability**: Easy to add new hosts
