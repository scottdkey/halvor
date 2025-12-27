# Halvor Agent Mesh Architecture

## Overview

The Halvor Agent Mesh is a decentralized peer-to-peer network that enables secure communication and data synchronization between Halvor agents running on different machines. Each agent can join the mesh and automatically discover and sync with all other peers.

## Key Features

- **Decentralized**: No single point of failure - any peer can act as an entry point
- **Auto-discovery**: Agents automatically find each other via Tailscale
- **Full mesh synchronization**: All peers know about all other peers
- **Flexible messaging**: Support for database sync, file transfer, media streaming, and custom data
- **Secure**: All communication can be encrypted with peer-specific shared secrets

## Architecture Components

### 1. Join Process

When a new agent joins the mesh:

1. **Request join token** from any existing peer
2. **Connect** to the peer using the token
3. **Receive** full list of all mesh peers
4. **Store** all peers in local database
5. **Broadcast** new peer info to all existing members
6. **Sync** with all peers to get latest data

```
┌─────────┐    join token    ┌─────────┐
│  Mint   │ ───────────────> │ Baulder │
│  (new)  │                  │ (peer)  │
└─────────┘                  └─────────┘
     │                             │
     │ ← peer list (oak, frigg)    │
     │                             │
     ├────────────┬────────────────┤
     │            │                │
     │            ↓                ↓
     │      ┌─────────┐      ┌─────────┐
     └─────>│   Oak   │      │  Frigg  │
            │ (peer)  │      │ (peer)  │
            └─────────┘      └─────────┘
                 │                │
                 ← gets notified  ←
                   about mint
```

### 2. Mesh Protocol

All communication uses a structured message format defined in [`mesh_protocol.rs`](../projects/core/agent/mesh_protocol.rs):

```rust
struct MeshMessage {
    message_id: String,      // Unique ID for deduplication
    from: String,            // Sender hostname
    to: String,              // Recipient (or "broadcast")
    payload: MessagePayload, // The actual data
    encryption: Option<...>, // Encryption metadata
    timestamp: i64,          // Unix timestamp
}
```

### 3. Message Types

The mesh supports multiple payload types:

#### Database Sync
Synchronize SQLite database changes across all peers:
```rust
MessagePayload::DatabaseSync {
    table: "agent_peers",
    operation: DbOperation::Insert,
    data: json!({"hostname": "mint", ...})
}
```

#### File Transfer
Transfer files in chunks (supports large files):
```rust
MessagePayload::FileTransfer {
    path: "configs/nginx.conf",
    chunk_index: 0,
    chunk_data: "base64...",
    checksum: "sha256..."
}
```

#### Media Streaming
Stream audio/video for playback:
```rust
MessagePayload::MediaStream {
    stream_id: "video-123",
    media_type: MediaType::Video,
    codec: "h264",
    sequence: 42,
    data: "base64..."
}
```

#### Config Update
Sync configuration changes:
```rust
MessagePayload::ConfigUpdate {
    key: "nginx.port",
    value: json!(8080),
    version: 5
}
```

#### Custom JSON
Application-specific data:
```rust
MessagePayload::CustomJson {
    app_type: "monitoring",
    data: json!({"cpu": 45, "mem": 2048})
}
```

### 4. Synchronization

Agents periodically sync with all known peers using `halvor agent sync`:

```
┌─────────┐
│  Mint   │
└────┬────┘
     │
     ├────────> [1/3] Sync host info
     ├────────> [2/3] Sync encrypted data
     └────────> [3/3] Sync mesh peers
                       ↓
                 Discover all agents
                 Add new peers to database
                 Update last_seen timestamps
```

### 5. Database Schema

Each agent maintains a local SQLite database with:

**`agent_peers` table**:
- `hostname`: Peer hostname (unique)
- `tailscale_ip`: Tailscale IP address
- `tailscale_hostname`: Tailscale FQDN
- `public_key`: Peer's public key
- `status`: active/inactive
- `last_seen_at`: Last contact timestamp
- `joined_at`: When peer joined mesh

**`peer_keys` table**:
- `peer_hostname`: Reference to agent_peers
- `shared_secret`: Encrypted communication key
- `algorithm`: Encryption algorithm

**`join_tokens` table**:
- `token`: Base64-encoded join token
- `issuer_hostname`: Who issued the token
- `expires_at`: Expiration timestamp
- `used`: Whether token was used
- `used_by_hostname`: Who used it

## Usage

### Setting up the mesh

1. **On the first machine** (e.g., baulder):
   ```bash
   # Start the agent
   halvor agent start --daemon

   # Generate a join token
   halvor agent token
   ```

2. **On other machines** (e.g., mint, oak):
   ```bash
   # Join using the token
   halvor agent join <token-from-baulder>
   ```

3. **Verify mesh status**:
   ```bash
   halvor agent peers
   halvor status
   ```

### Syncing the mesh

To ensure all peers know about each other:

```bash
# On any machine
halvor agent sync
```

This will:
- Discover all reachable agents via Tailscale
- Add any new peers to the local database
- Update last_seen timestamps
- Sync configuration and encrypted data

### Updating hostname

When a machine's hostname changes:

```bash
halvor agent hostname new-name
```

This will:
- Update local database
- Notify all mesh peers
- Update peer databases across the mesh

## Security

- **Join tokens** expire after 24 hours
- **Tokens are single-use** - marked as used after join
- **Shared secrets** are generated per-peer relationship
- **Encryption** can be enabled per-message
- **Tailscale** provides encrypted transport layer

## Future Enhancements

- [ ] Automatic background sync (currently manual)
- [ ] Conflict resolution for concurrent database updates
- [ ] Compressed message transfer for large payloads
- [ ] Streaming protocol for real-time media
- [ ] Message acknowledgment and retry logic
- [ ] Peer health monitoring and automatic removal
- [ ] Multi-hop routing for offline peers

## Troubleshooting

### Peers not discovering each other

1. Check Tailscale status: `tailscale status`
2. Verify agent is running: `halvor agent status`
3. Check firewall allows port 13500: `sudo ufw status`
4. View agent logs: `sudo journalctl -u halvor-agent.service -f`

### Database out of sync

Run sync manually:
```bash
halvor agent sync --force
```

### Join token errors

- **"Token expired"**: Generate a new token
- **"Already used"**: Generate a new token
- **"Invalid token"**: Verify agent is running on issuing machine

## Implementation Files

- `projects/core/agent/mesh.rs` - Join token generation/validation
- `projects/core/agent/mesh_protocol.rs` - Message protocol definitions
- `projects/core/agent/server.rs` - Agent server and join handling
- `projects/core/agent/sync.rs` - Synchronization logic
- `projects/core/agent/discovery.rs` - Peer discovery via Tailscale
- `projects/core/commands/agent.rs` - CLI commands
- `projects/core/db/migrations/005_add_agent_mesh_tables.rs` - Database schema
