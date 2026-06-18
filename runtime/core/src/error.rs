//! APEX Error Taxonomy
//!
//! All errors in the APEX runtime are typed, structured, and carry enough
//! context to enable actionable diagnostics. No opaque strings.

use std::fmt;

/// The canonical result type for all APEX operations.
pub type ApexResult<T> = Result<T, ApexError>;

/// Structured error taxonomy for the APEX runtime.
///
/// Each variant maps to a distinct failure domain. Errors must not leak
/// internal implementation details and must never contain sensitive data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApexError {
    // --- Identity & Registry ---
    /// A node with the same ID or name already exists in the registry.
    NodeAlreadyRegistered { name: String },
    /// The referenced node was not found in the registry.
    NodeNotFound { id: String },
    /// A topic with the same canonical name already exists.
    TopicAlreadyRegistered { name: String },
    /// The referenced topic was not found.
    TopicNotFound { name: String },

    // --- Transport ---
    /// Transport initialization failed.
    TransportInitFailed { kind: String, reason: String },
    /// A transport operation exceeded its deadline.
    TransportTimeout { kind: String, elapsed_ms: u64 },
    /// The transport is not available or was closed.
    TransportUnavailable { kind: String },
    /// Connection to a peer was refused.
    PeerConnectionRefused { peer_id: String, reason: String },

    // --- Serialization ---
    /// The payload could not be serialized.
    SerializeFailed { type_name: String, reason: String },
    /// The payload could not be deserialized.
    DeserializeFailed { type_name: String, reason: String },
    /// The received payload is malformed or truncated.
    MalformedPayload { reason: String },
    /// Schema hash or version mismatch detected.
    SchemaMismatch { expected_hash: u64, received_hash: u64 },

    // --- QoS ---
    /// Publisher and subscriber QoS are incompatible.
    QosMismatch { reason: String },
    /// A reliable message exceeded all retry attempts.
    DeliveryFailed { sequence: u64, reason: String },
    /// A queue reached its bounded limit and cannot accept more messages.
    QueueSaturated { topic: String, queue_limit: usize },

    // --- Memory ---
    /// The memory pool is exhausted; no buffers available.
    PoolExhausted { pool_name: String },
    /// A loaned buffer was held beyond its timeout.
    LoanTimeout { loan_id: String, timeout_ms: u64 },
    /// A buffer is in an invalid ownership state.
    InvalidBufferOwnership { reason: String },

    // --- Discovery ---
    /// Peer discovery failed completely (all strategies exhausted).
    DiscoveryFailed { reason: String },
    /// A peer was marked as stale and removed from the active set.
    PeerStale { peer_id: String },
    /// Capability negotiation between two peers failed.
    CapabilityNegotiationFailed { reason: String },

    // --- Config ---
    /// Configuration file could not be loaded.
    ConfigLoadFailed { path: String, reason: String },
    /// A required configuration field is missing.
    ConfigMissingField { field: String },
    /// A configuration value failed validation.
    ConfigInvalidValue { field: String, reason: String },

    // --- Lifecycle ---
    /// An operation was attempted on a node that has not been initialized.
    NodeNotInitialized,
    /// An operation was attempted after shutdown was initiated.
    ShutdownInProgress,
    /// A node failed to reach ready state within the expected window.
    StartupTimeout { node_name: String, timeout_ms: u64 },

    // --- Security ---
    /// A node identity could not be verified.
    IdentityVerificationFailed { reason: String },
    /// The caller does not have permission to perform this operation.
    Unauthorized { operation: String },

    // --- Internal ---
    /// An invariant was violated. This is always a bug.
    InvariantViolation { message: String },
    /// An I/O error from the OS layer.
    Io { reason: String },
}

impl fmt::Display for ApexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApexError::NodeAlreadyRegistered { name } =>
                write!(f, "node already registered: {name}"),
            ApexError::NodeNotFound { id } =>
                write!(f, "node not found: {id}"),
            ApexError::TopicAlreadyRegistered { name } =>
                write!(f, "topic already registered: {name}"),
            ApexError::TopicNotFound { name } =>
                write!(f, "topic not found: {name}"),
            ApexError::TransportInitFailed { kind, reason } =>
                write!(f, "transport init failed [{kind}]: {reason}"),
            ApexError::TransportTimeout { kind, elapsed_ms } =>
                write!(f, "transport timeout [{kind}] after {elapsed_ms}ms"),
            ApexError::TransportUnavailable { kind } =>
                write!(f, "transport unavailable: {kind}"),
            ApexError::PeerConnectionRefused { peer_id, reason } =>
                write!(f, "peer connection refused [{peer_id}]: {reason}"),
            ApexError::SerializeFailed { type_name, reason } =>
                write!(f, "serialize failed [{type_name}]: {reason}"),
            ApexError::DeserializeFailed { type_name, reason } =>
                write!(f, "deserialize failed [{type_name}]: {reason}"),
            ApexError::MalformedPayload { reason } =>
                write!(f, "malformed payload: {reason}"),
            ApexError::SchemaMismatch { expected_hash, received_hash } =>
                write!(f, "schema mismatch: expected={expected_hash:#x} received={received_hash:#x}"),
            ApexError::QosMismatch { reason } =>
                write!(f, "QoS mismatch: {reason}"),
            ApexError::DeliveryFailed { sequence, reason } =>
                write!(f, "delivery failed [seq={sequence}]: {reason}"),
            ApexError::QueueSaturated { topic, queue_limit } =>
                write!(f, "queue saturated [topic={topic}, limit={queue_limit}]"),
            ApexError::PoolExhausted { pool_name } =>
                write!(f, "memory pool exhausted: {pool_name}"),
            ApexError::LoanTimeout { loan_id, timeout_ms } =>
                write!(f, "buffer loan timeout [id={loan_id}] after {timeout_ms}ms"),
            ApexError::InvalidBufferOwnership { reason } =>
                write!(f, "invalid buffer ownership: {reason}"),
            ApexError::DiscoveryFailed { reason } =>
                write!(f, "discovery failed: {reason}"),
            ApexError::PeerStale { peer_id } =>
                write!(f, "peer stale and removed: {peer_id}"),
            ApexError::CapabilityNegotiationFailed { reason } =>
                write!(f, "capability negotiation failed: {reason}"),
            ApexError::ConfigLoadFailed { path, reason } =>
                write!(f, "config load failed [{path}]: {reason}"),
            ApexError::ConfigMissingField { field } =>
                write!(f, "config missing required field: {field}"),
            ApexError::ConfigInvalidValue { field, reason } =>
                write!(f, "config invalid value [{field}]: {reason}"),
            ApexError::NodeNotInitialized =>
                write!(f, "node not initialized"),
            ApexError::ShutdownInProgress =>
                write!(f, "shutdown in progress, operation rejected"),
            ApexError::StartupTimeout { node_name, timeout_ms } =>
                write!(f, "startup timeout [{node_name}] after {timeout_ms}ms"),
            ApexError::IdentityVerificationFailed { reason } =>
                write!(f, "identity verification failed: {reason}"),
            ApexError::Unauthorized { operation } =>
                write!(f, "unauthorized operation: {operation}"),
            ApexError::InvariantViolation { message } =>
                write!(f, "INVARIANT VIOLATION (bug): {message}"),
            ApexError::Io { reason } =>
                write!(f, "I/O error: {reason}"),
        }
    }
}

impl std::error::Error for ApexError {}

impl From<std::io::Error> for ApexError {
    fn from(e: std::io::Error) -> Self {
        ApexError::Io { reason: e.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_node_not_found() {
        let e = ApexError::NodeNotFound { id: "abc-123".into() };
        assert_eq!(e.to_string(), "node not found: abc-123");
    }

    #[test]
    fn error_display_schema_mismatch() {
        let e = ApexError::SchemaMismatch {
            expected_hash: 0xDEAD,
            received_hash: 0xBEEF,
        };
        let s = e.to_string();
        assert!(s.contains("schema mismatch"));
        assert!(s.contains("0xdead"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ApexError>();
    }
}
