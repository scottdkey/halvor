//! Mesh protocol for peer-to-peer communication
//!
//! This module defines the protocol for communication between mesh peers.
//! It supports various message types including:
//! - Database synchronization
//! - File transfers
//! - Streaming media (audio/video)
//! - Configuration updates
//! - Custom JSON payloads

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum size for a single message chunk (16MB)
pub const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Mesh message envelope - wraps all messages sent between peers
#[derive(Debug, Serialize, Deserialize)]
pub struct MeshMessage {
    /// Unique message ID for tracking and deduplication
    pub message_id: String,

    /// Sender hostname
    pub from: String,

    /// Target hostname (or "broadcast" for all peers)
    pub to: String,

    /// Message type and payload
    pub payload: MessagePayload,

    /// Optional encryption metadata
    pub encryption: Option<EncryptionMetadata>,

    /// Message timestamp (Unix timestamp)
    pub timestamp: i64,
}

/// Different types of payloads that can be sent between peers
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessagePayload {
    /// Database sync - send SQLite changes
    DatabaseSync {
        /// Table name
        table: String,
        /// Operation (insert, update, delete)
        operation: DbOperation,
        /// Row data as JSON
        data: serde_json::Value,
    },

    /// File transfer (chunked for large files)
    FileTransfer {
        /// File path relative to a known base
        path: String,
        /// Total file size in bytes
        total_size: u64,
        /// Current chunk number (0-indexed)
        chunk_index: u32,
        /// Total number of chunks
        total_chunks: u32,
        /// Chunk data (base64 encoded)
        chunk_data: String,
        /// SHA-256 checksum of complete file
        checksum: String,
    },

    /// Media streaming (audio/video)
    MediaStream {
        /// Stream ID for tracking
        stream_id: String,
        /// Media type (audio/video)
        media_type: MediaType,
        /// Codec information
        codec: String,
        /// Sequence number for ordering
        sequence: u64,
        /// Frame/chunk data (base64 encoded)
        data: String,
        /// Is this the last chunk?
        is_final: bool,
    },

    /// Configuration update
    ConfigUpdate {
        /// Config key
        key: String,
        /// Config value (JSON)
        value: serde_json::Value,
        /// Version for conflict resolution
        version: u64,
    },

    /// Generic JSON payload for custom applications
    CustomJson {
        /// Application-specific type identifier
        app_type: String,
        /// JSON data
        data: serde_json::Value,
    },

    /// Ping/heartbeat
    Ping,

    /// Pong response
    Pong,

    /// Acknowledgment
    Ack {
        /// ID of message being acknowledged
        ack_message_id: String,
    },

    /// Error response
    Error {
        /// Error code
        code: String,
        /// Error message
        message: String,
    },
}

/// Database operations
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum DbOperation {
    Insert,
    Update,
    Delete,
}

/// Media types for streaming
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
}

/// Encryption metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Algorithm used (e.g., "aes-256-gcm")
    pub algorithm: String,
    /// Initialization vector (base64)
    pub iv: String,
    /// Authentication tag (base64)
    pub tag: String,
}

impl MeshMessage {
    /// Create a new message
    pub fn new(from: String, to: String, payload: MessagePayload) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            payload,
            encryption: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a database sync message
    pub fn database_sync(
        from: String,
        to: String,
        table: String,
        operation: DbOperation,
        data: serde_json::Value,
    ) -> Self {
        Self::new(
            from,
            to,
            MessagePayload::DatabaseSync {
                table,
                operation,
                data,
            },
        )
    }

    /// Create a config update message
    pub fn config_update(
        from: String,
        to: String,
        key: String,
        value: serde_json::Value,
        version: u64,
    ) -> Self {
        Self::new(
            from,
            to,
            MessagePayload::ConfigUpdate { key, value, version },
        )
    }

    /// Create a ping message
    pub fn ping(from: String, to: String) -> Self {
        Self::new(from, to, MessagePayload::Ping)
    }

    /// Create a pong response
    pub fn pong(from: String, to: String) -> Self {
        Self::new(from, to, MessagePayload::Pong)
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize to bytes (for efficient transport)
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Message router for handling incoming messages
pub struct MessageRouter {
    handlers: HashMap<String, Box<dyn MessageHandler>>,
}

/// Trait for handling specific message types
pub trait MessageHandler: Send + Sync {
    fn handle(&self, message: &MeshMessage) -> Result<Option<MeshMessage>, anyhow::Error>;
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a specific message type
    pub fn register<H: MessageHandler + 'static>(&mut self, message_type: String, handler: H) {
        self.handlers.insert(message_type, Box::new(handler));
    }

    /// Route an incoming message to the appropriate handler
    pub fn route(&self, message: &MeshMessage) -> Result<Option<MeshMessage>, anyhow::Error> {
        let message_type = match &message.payload {
            MessagePayload::DatabaseSync { .. } => "database_sync",
            MessagePayload::FileTransfer { .. } => "file_transfer",
            MessagePayload::MediaStream { .. } => "media_stream",
            MessagePayload::ConfigUpdate { .. } => "config_update",
            MessagePayload::CustomJson { .. } => "custom_json",
            MessagePayload::Ping => "ping",
            MessagePayload::Pong => "pong",
            MessagePayload::Ack { .. } => "ack",
            MessagePayload::Error { .. } => "error",
        };

        if let Some(handler) = self.handlers.get(message_type) {
            handler.handle(message)
        } else {
            Ok(None)
        }
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = MeshMessage::ping("alice".to_string(), "bob".to_string());
        let json = msg.to_json().unwrap();
        let deserialized = MeshMessage::from_json(&json).unwrap();

        assert_eq!(msg.from, deserialized.from);
        assert_eq!(msg.to, deserialized.to);
        matches!(deserialized.payload, MessagePayload::Ping);
    }

    #[test]
    fn test_database_sync_message() {
        let data = serde_json::json!({"id": "123", "name": "test"});
        let msg = MeshMessage::database_sync(
            "node1".to_string(),
            "node2".to_string(),
            "users".to_string(),
            DbOperation::Insert,
            data,
        );

        let json = msg.to_json().unwrap();
        let deserialized = MeshMessage::from_json(&json).unwrap();

        if let MessagePayload::DatabaseSync { table, operation, .. } = deserialized.payload {
            assert_eq!(table, "users");
            assert_eq!(operation, DbOperation::Insert);
        } else {
            panic!("Wrong payload type");
        }
    }
}
