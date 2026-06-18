//! APEX Health Subsystem
//!
//! Every node and peer in APEX has observable health state.
//! Health is not just a binary alive/dead flag.
//! It is a machine with meaningful states and transitions.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// The combined health status of a node or subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Node is fully operational.
    Healthy,
    /// Node is running but with reduced capability or higher latency.
    /// Operations should proceed with awareness.
    Degraded { reason: String },
    /// Node is temporarily unavailable but expected to recover.
    Recovering { reason: String },
    /// Node has failed and will not recover without intervention.
    Failed { reason: String },
    /// Node is shutting down cleanly.
    ShuttingDown,
    /// Health state has not yet been determined (startup).
    Unknown,
}

impl HealthStatus {
    /// Returns true if the node is usable (healthy or degraded).
    pub fn is_usable(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded { .. })
    }

    /// Returns true if the node is operational at full capacity.
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthStatus::Healthy)
    }

    /// Returns a short string label for logging and metrics.
    pub fn label(&self) -> &'static str {
        match self {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Degraded { .. } => "degraded",
            HealthStatus::Recovering { .. } => "recovering",
            HealthStatus::Failed { .. } => "failed",
            HealthStatus::ShuttingDown => "shutting_down",
            HealthStatus::Unknown => "unknown",
        }
    }
}

/// Liveliness state of a node or publisher.
///
/// Liveliness is separate from health: a node can be alive but degraded,
/// or considered stale by the discovery subsystem without being failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivelinessState {
    /// Actively sending heartbeats or messages.
    Alive,
    /// No recent signal; may be paused or overloaded.
    Stale,
    /// Has explicitly announced it is leaving or shutting down.
    Left,
    /// No signal within the liveliness timeout. Presumed gone.
    Dead,
}

impl LivelinessState {
    pub fn label(&self) -> &'static str {
        match self {
            LivelinessState::Alive => "alive",
            LivelinessState::Stale => "stale",
            LivelinessState::Left => "left",
            LivelinessState::Dead => "dead",
        }
    }
}

/// Readiness state: is the node ready to process and route messages?
///
/// Readiness is about capability, not liveness.
/// A node can be alive but not yet ready (still initializing, discovery pending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessState {
    /// Fully ready to accept and process messages.
    Ready,
    /// Still initializing. Not ready to accept messages.
    Initializing,
    /// Discovery pending. Runtime partially initialized.
    DiscoveryPending,
    /// Not ready due to a recoverable condition.
    NotReady { code: u32 },
}

impl ReadinessState {
    pub fn is_ready(&self) -> bool {
        matches!(self, ReadinessState::Ready)
    }

    pub fn label(&self) -> &'static str {
        match self {
            ReadinessState::Ready => "ready",
            ReadinessState::Initializing => "initializing",
            ReadinessState::DiscoveryPending => "discovery_pending",
            ReadinessState::NotReady { .. } => "not_ready",
        }
    }
}

/// Snapshot of a node's health at a specific moment in time.
/// Used for diagnostics, dashboards, and audit trails.
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    /// Timestamp when the snapshot was taken (Unix nanos).
    pub timestamp_ns: u128,
    /// Current health status.
    pub status: HealthStatus,
    /// Liveliness state.
    pub liveliness: LivelinessState,
    /// Readiness state.
    pub readiness: ReadinessState,
    /// Number of successful health checks since last reset.
    pub healthy_ticks: u64,
    /// Number of degraded or failed health checks since last reset.
    pub unhealthy_ticks: u64,
    /// Total uptime in seconds since node started.
    pub uptime_secs: u64,
}

impl HealthSnapshot {
    /// Create a new snapshot with the current system time.
    pub fn now(
        status: HealthStatus,
        liveliness: LivelinessState,
        readiness: ReadinessState,
        healthy_ticks: u64,
        unhealthy_ticks: u64,
        uptime_secs: u64,
    ) -> Self {
        let timestamp_ns = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        Self {
            timestamp_ns,
            status,
            liveliness,
            readiness,
            healthy_ticks,
            unhealthy_ticks,
            uptime_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_usability() {
        assert!(HealthStatus::Healthy.is_usable());
        assert!(HealthStatus::Degraded { reason: "memory pressure".into() }.is_usable());
        assert!(!HealthStatus::Failed { reason: "crash".into() }.is_usable());
        assert!(!HealthStatus::Unknown.is_usable());
    }

    #[test]
    fn health_status_labels() {
        assert_eq!(HealthStatus::Healthy.label(), "healthy");
        assert_eq!(HealthStatus::ShuttingDown.label(), "shutting_down");
    }

    #[test]
    fn liveliness_labels() {
        assert_eq!(LivelinessState::Alive.label(), "alive");
        assert_eq!(LivelinessState::Dead.label(), "dead");
    }

    #[test]
    fn readiness_is_ready() {
        assert!(ReadinessState::Ready.is_ready());
        assert!(!ReadinessState::Initializing.is_ready());
    }
}
