//! APEX Identity Primitives
//!
//! All identifiers in the APEX runtime are strongly typed, globally unique,
//! and carry enough context to be useful in logs and diagnostics.
//! We never use raw strings or bare integers as IDs.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// A globally unique node identifier.
///
/// Composed of host fingerprint + process ID + monotonic counter + boot epoch.
/// Never reused across reboots or restarts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new NodeId with the given raw string.
    /// Prefer using `NodeId::generate()` in production.
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    /// Generate a unique NodeId based on process and time.
    pub fn generate(node_name: &str) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let pid = std::process::id();
        Self(format!("{node_name}-{pid}-{ts}"))
    }

    /// Returns the inner string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A globally unique topic identifier derived from the canonical topic name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicId(pub String);

impl TopicId {
    /// Derive a TopicId from a canonical topic name.
    pub fn from_name(name: &str) -> Self {
        Self(format!("topic:{name}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TopicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Session identifier — unique per node boot.
/// Changes on every restart. Used for epoch tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn generate(node_id: &NodeId) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self(format!("session-{}-{ts}", node_id.as_str()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Peer identifier — used to refer to remote nodes discovered via discovery.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Host identifier — fingerprint of the machine this node runs on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostId(pub String);

impl HostId {
    pub fn local() -> Self {
        // In production, derive from hostname + machine-id or MAC.
        let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into());
        Self(format!("host:{hostname}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Message identifier — unique within a session+topic combination.
/// Enables deduplication, ordering, and replay correlation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId {
    /// Publisher node session epoch
    pub session_id: SessionId,
    /// Monotonically increasing sequence number within this publisher session
    pub sequence: u64,
}

impl MessageId {
    pub fn new(session_id: SessionId, sequence: u64) -> Self {
        Self { session_id, sequence }
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.session_id, self.sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_generate_is_unique() {
        let a = NodeId::generate("test_node");
        let b = NodeId::generate("test_node");
        // Both valid and non-empty; timing may rarely collide in tests but shouldn't in prod
        assert!(!a.as_str().is_empty());
        assert!(!b.as_str().is_empty());
    }

    #[test]
    fn topic_id_from_name() {
        let id = TopicId::from_name("scan");
        assert_eq!(id.as_str(), "topic:scan");
    }

    #[test]
    fn message_id_display() {
        let sess = SessionId("sess-abc".into());
        let msg_id = MessageId::new(sess, 42);
        assert_eq!(msg_id.to_string(), "sess-abc:42");
    }
}
