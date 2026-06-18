//! APEX Peer Record
//!
//! A peer is any remote APEX node discovered via any discovery strategy.
//! Peers are tracked in a PeerTable with explicit state transitions.
//! No peer is ever silently removed: all removals are logged and auditable.

use std::time::{Duration, Instant};
use std::net::SocketAddr;
use std::collections::HashSet;

/// The current operational state of a discovered peer.
///
/// Transitions:
/// Discovered -> Active (on successful capability negotiation)
/// Active -> Stale (on heartbeat timeout)
/// Active -> Quarantined (on repeated failures)
/// Stale -> Active (on heartbeat received)
/// Stale -> Dead (on stale_ttl exceeded)
/// Quarantined -> Active (on manual recovery or backoff)
/// Any -> Left (on explicit leave message received)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Just discovered; negotiation in progress.
    Discovered,
    /// Fully negotiated and active.
    Active,
    /// Heartbeat missed. Monitoring closely.
    Stale,
    /// Repeated failures. Isolated from hot path.
    Quarantined { reason: String },
    /// Explicitly said goodbye or cleanly disconnected.
    Left,
    /// Exceeded stale TTL. Presumed gone. Will be garbage collected.
    Dead,
}

impl PeerState {
    pub fn is_usable(&self) -> bool {
        matches!(self, PeerState::Active)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PeerState::Discovered => "discovered",
            PeerState::Active => "active",
            PeerState::Stale => "stale",
            PeerState::Quarantined { .. } => "quarantined",
            PeerState::Left => "left",
            PeerState::Dead => "dead",
        }
    }
}

/// How visible a peer is in the current topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerVisibility {
    /// Directly reachable on the same host (shared memory available).
    LocalProcess,
    /// Reachable on same subnet.
    LocalNetwork,
    /// Reachable via routed network.
    Remote,
    /// Reachability not yet determined.
    Unknown,
}

impl PeerVisibility {
    /// Returns true if shared memory transport is eligible.
    pub fn allows_shm(&self) -> bool {
        matches!(self, PeerVisibility::LocalProcess)
    }

    pub fn label(&self) -> &'static str {
        match self {
            PeerVisibility::LocalProcess => "local_process",
            PeerVisibility::LocalNetwork => "local_network",
            PeerVisibility::Remote => "remote",
            PeerVisibility::Unknown => "unknown",
        }
    }
}

/// A complete record of a discovered peer.
///
/// This record is the source of truth for everything the local node
/// knows about a remote peer. It drives transport selection, QoS matching,
/// capability negotiation, and health monitoring.
#[derive(Debug, Clone)]
pub struct PeerRecord {
    /// Globally unique peer identifier.
    pub peer_id: String,
    /// Human-readable node name.
    pub node_name: String,
    /// Current state in the peer state machine.
    pub state: PeerState,
    /// How visible/reachable this peer is.
    pub visibility: PeerVisibility,
    /// Primary network address for this peer.
    pub address: Option<SocketAddr>,
    /// APEX runtime version reported by this peer.
    pub runtime_version: String,
    /// Set of topic names this peer publishes.
    pub published_topics: HashSet<String>,
    /// Set of topic names this peer subscribes to.
    pub subscribed_topics: HashSet<String>,
    /// When this record was first created.
    pub first_seen: Instant,
    /// When this record was last updated (heartbeat or message).
    pub last_seen: Instant,
    /// Number of consecutive heartbeat misses.
    pub missed_heartbeats: u32,
    /// Number of times this peer has been quarantined.
    pub quarantine_count: u32,
    /// How this peer was discovered.
    pub discovery_strategy: String,
}

impl PeerRecord {
    /// Create a new peer record in Discovered state.
    pub fn new(
        peer_id: String,
        node_name: String,
        address: Option<SocketAddr>,
        runtime_version: String,
        discovery_strategy: String,
    ) -> Self {
        let now = Instant::now();
        Self {
            peer_id,
            node_name,
            state: PeerState::Discovered,
            visibility: PeerVisibility::Unknown,
            address,
            runtime_version,
            published_topics: HashSet::new(),
            subscribed_topics: HashSet::new(),
            first_seen: now,
            last_seen: now,
            missed_heartbeats: 0,
            quarantine_count: 0,
            discovery_strategy,
        }
    }

    /// Update last_seen and reset heartbeat counter.
    pub fn heartbeat_received(&mut self) {
        self.last_seen = Instant::now();
        self.missed_heartbeats = 0;
        if self.state == PeerState::Stale {
            self.state = PeerState::Active;
        }
    }

    /// Record a missed heartbeat. Returns true if the peer should become Stale.
    pub fn heartbeat_missed(&mut self, stale_after: u32) -> bool {
        self.missed_heartbeats += 1;
        if self.missed_heartbeats >= stale_after && self.state == PeerState::Active {
            self.state = PeerState::Stale;
            return true;
        }
        false
    }

    /// Mark the peer as having sent a leave notice.
    pub fn mark_left(&mut self) {
        self.state = PeerState::Left;
    }

    /// Mark the peer as dead (TTL exceeded).
    pub fn mark_dead(&mut self) {
        self.state = PeerState::Dead;
    }

    /// Elapsed time since last heartbeat.
    pub fn elapsed_since_last_seen(&self) -> Duration {
        self.last_seen.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_peer(id: &str) -> PeerRecord {
        PeerRecord::new(
            id.into(),
            "test_node".into(),
            None,
            "0.1.0".into(),
            "multicast".into(),
        )
    }

    #[test]
    fn new_peer_is_discovered() {
        let p = make_peer("peer-1");
        assert_eq!(p.state, PeerState::Discovered);
        assert!(!p.state.is_usable());
    }

    #[test]
    fn heartbeat_clears_stale() {
        let mut p = make_peer("peer-2");
        p.state = PeerState::Active;
        p.missed_heartbeats = 2;
        let became_stale = p.heartbeat_missed(3);
        assert!(became_stale);
        assert_eq!(p.state, PeerState::Stale);
        p.heartbeat_received();
        assert_eq!(p.state, PeerState::Active);
        assert_eq!(p.missed_heartbeats, 0);
    }

    #[test]
    fn mark_dead_transitions() {
        let mut p = make_peer("peer-3");
        p.mark_dead();
        assert_eq!(p.state, PeerState::Dead);
        assert!(!p.state.is_usable());
    }
}
