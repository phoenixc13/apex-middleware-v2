//! APEX Adaptive Discovery
//!
//! Discovery in APEX is never a single strategy. It is a layered system:
//!
//! 1. Multicast bootstrap (LAN, UDP)
//! 2. Static peer fallback (when multicast fails or is disabled)
//! 3. Unicast bootstrap list
//! 4. Localhost-only mode (dev)
//! 5. Manual topology mode (full control)
//! 6. Degraded discovery mode (partial topology accepted)
//!
//! Wi-Fi and unstable networks are treated as first-class, not exceptions.
//! Discovery never assumes all peers will be found. The system must degrade
//! gracefully when only a subset of peers are visible.

pub mod peer;
pub mod strategy;
pub mod table;
pub mod hello;
pub mod ttl;

pub use peer::{PeerRecord, PeerVisibility, PeerState};
pub use strategy::{DiscoveryMode, DiscoveryConfig};
pub use table::PeerTable;
pub use hello::{HelloPacket, HelloResponse};
