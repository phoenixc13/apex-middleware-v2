//! APEX Middleware — Core Runtime Library
//!
//! This is the foundation crate. It defines all primitive types, identifiers,
//! error taxonomy, and core contracts used across the entire APEX runtime.
//!
//! Rules:
//! - No framework dependencies
//! - No I/O side effects
//! - No allocator assumptions beyond std
//! - All types must be Clone, Debug, and Send + Sync where applicable

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(missing_docs)]

pub mod error;
pub mod identity;
pub mod node;
pub mod topic;
pub mod message;
pub mod qos;
pub mod transport;
pub mod capability;
pub mod health;
pub mod config;
pub mod profile;
pub mod version;
pub mod metrics;

pub use error::{ApexError, ApexResult};
pub use identity::{NodeId, TopicId, SessionId, PeerId, HostId, MessageId};
pub use node::{NodeName, NodeState, NodeInfo};
pub use topic::{TopicName, TopicInfo, SchemaHash, SchemaVersion};
pub use message::{MessageHeader, MessageFlags};
pub use qos::{Qos, Reliability, History, LateJoinPolicy, CongestPolicy, DropPolicy};
pub use transport::{TransportKind, TransportHint};
pub use capability::{CapabilitySet, Capability};
pub use health::{HealthStatus, LivelinessState, ReadinessState};
pub use config::ApexConfig;
pub use profile::DeploymentProfile;
pub use version::RuntimeVersion;
