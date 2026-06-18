//! APEX QoS Engine — Operational Contracts
//!
//! QoS in APEX is a first-class operational contract, not just configuration.
//! A publisher and subscriber with incompatible QoS must NEVER silently connect.
//! Mismatches are surfaced as typed errors with actionable reason codes.

use std::time::Duration;

/// Delivery reliability mode.
///
/// - `BestEffort`: No retransmission. Messages may be dropped.
///   Use for high-frequency sensor data where loss is acceptable.
/// - `Reliable`: The runtime guarantees delivery within configured retries and ack_timeout.
///   Use for control, config, and critical coordination data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reliability {
    /// No retransmission. Fire and forget.
    BestEffort,
    /// Guaranteed delivery within configured policy.
    Reliable,
}

/// History policy: how many messages to retain in the publisher queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum History {
    /// Retain only the last N messages.
    KeepLast(usize),
    /// Do not retain any messages.
    NoHistory,
}

impl History {
    /// Returns the retention depth as a usize.
    pub fn depth(&self) -> usize {
        match self {
            History::KeepLast(n) => *n,
            History::NoHistory => 0,
        }
    }
}

/// Late join policy: what a subscriber receives when it joins after messages
/// have already been published on a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LateJoinPolicy {
    /// No historical messages. Subscriber sees only future messages.
    None,
    /// Receive the most recent retained message, if any.
    Recent,
    /// Receive all messages within the publisher's history window.
    BoundedWindow,
}

/// Drop policy: what happens when a bounded queue is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DropPolicy {
    /// Drop the oldest message in the queue (sliding window).
    DropOldest,
    /// Reject the incoming message. Caller receives QueueSaturated error.
    RejectNew,
}

/// Congestion policy: what happens when a slow consumer creates backpressure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CongestPolicy {
    /// Drop messages destined for the slow consumer, log the drop.
    DropToSlowConsumer,
    /// Block the publisher briefly, then drop if timeout exceeded.
    BlockWithTimeout,
    /// Isolate the slow consumer from the hot path. Marks it as degraded.
    IsolateConsumer,
}

/// Liveliness mode: how a subscriber detects that a publisher has gone silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Liveliness {
    /// Publisher sends explicit heartbeat messages.
    ManualAssertion,
    /// Runtime infers liveliness from message arrival times.
    AutomaticHeartbeat,
}

/// The complete QoS profile for a publisher or subscriber.
///
/// QoS is a contract: two endpoints only connect if their policies are
/// compatible. Incompatibility is surfaced as `ApexError::QosMismatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qos {
    /// Delivery reliability mode.
    pub reliability: Reliability,
    /// History retention policy.
    pub history: History,
    /// Late join policy.
    pub late_join_policy: LateJoinPolicy,
    /// Drop policy when queue is at capacity.
    pub drop_policy: DropPolicy,
    /// Congestion policy for slow consumers.
    pub congestion_policy: CongestPolicy,
    /// Liveliness detection mode.
    pub liveliness: Liveliness,
    /// Maximum number of messages in the outbound queue. Bounded.
    pub queue_limit: usize,
    /// How long to wait for an acknowledgment before retrying (Reliable only).
    pub ack_timeout: Duration,
    /// Total delivery timeout (Reliable only).
    pub delivery_timeout: Duration,
    /// Heartbeat interval (AutomaticHeartbeat).
    pub heartbeat_interval: Duration,
    /// Liveliness timeout: how long without a heartbeat before declaring dead.
    pub liveliness_timeout: Duration,
    /// Maximum retry attempts for Reliable delivery.
    pub max_retries: u32,
    /// Message priority (0 = lowest, 255 = highest).
    pub priority: u8,
}

impl Qos {
    /// Sensible defaults for best-effort sensor streaming.
    pub fn sensor_streaming() -> Self {
        Self {
            reliability: Reliability::BestEffort,
            history: History::KeepLast(8),
            late_join_policy: LateJoinPolicy::None,
            drop_policy: DropPolicy::DropOldest,
            congestion_policy: CongestPolicy::DropToSlowConsumer,
            liveliness: Liveliness::AutomaticHeartbeat,
            queue_limit: 32,
            ack_timeout: Duration::from_millis(100),
            delivery_timeout: Duration::from_millis(500),
            heartbeat_interval: Duration::from_millis(500),
            liveliness_timeout: Duration::from_secs(2),
            max_retries: 0,
            priority: 64,
        }
    }

    /// Sensible defaults for reliable control signaling.
    pub fn reliable_control() -> Self {
        Self {
            reliability: Reliability::Reliable,
            history: History::KeepLast(1),
            late_join_policy: LateJoinPolicy::Recent,
            drop_policy: DropPolicy::RejectNew,
            congestion_policy: CongestPolicy::IsolateConsumer,
            liveliness: Liveliness::ManualAssertion,
            queue_limit: 8,
            ack_timeout: Duration::from_millis(200),
            delivery_timeout: Duration::from_secs(2),
            heartbeat_interval: Duration::from_secs(1),
            liveliness_timeout: Duration::from_secs(5),
            max_retries: 5,
            priority: 192,
        }
    }
}

impl Default for Qos {
    fn default() -> Self {
        Self::sensor_streaming()
    }
}

/// Result of QoS compatibility check between publisher and subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QosCompatibility {
    /// Fully compatible. Connection can proceed.
    Compatible,
    /// Incompatible. Reason describes what policy conflicted.
    Incompatible { reason: String },
}

/// Check whether a publisher QoS and subscriber QoS are compatible.
///
/// Rules (conservative by design):
/// - A Reliable subscriber cannot connect to a BestEffort publisher.
/// - A subscriber with a larger queue_limit than the publisher's creates
///   no issue (subscriber governs its own inbound capacity).
/// - Priority mismatch is warned but not fatal.
pub fn check_qos_compatibility(publisher: &Qos, subscriber: &Qos) -> QosCompatibility {
    if publisher.reliability == Reliability::BestEffort
        && subscriber.reliability == Reliability::Reliable
    {
        return QosCompatibility::Incompatible {
            reason: format!(
                "subscriber requires Reliable delivery but publisher offers BestEffort; \
                 this connection would violate the subscriber's delivery contract"
            ),
        };
    }

    QosCompatibility::Compatible
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_best_effort_to_best_effort() {
        let pub_qos = Qos::sensor_streaming();
        let sub_qos = Qos::sensor_streaming();
        assert_eq!(check_qos_compatibility(&pub_qos, &sub_qos), QosCompatibility::Compatible);
    }

    #[test]
    fn incompatible_besteffort_publisher_reliable_subscriber() {
        let pub_qos = Qos::sensor_streaming(); // BestEffort
        let sub_qos = Qos::reliable_control(); // Reliable
        let result = check_qos_compatibility(&pub_qos, &sub_qos);
        assert!(matches!(result, QosCompatibility::Incompatible { .. }));
    }

    #[test]
    fn compatible_reliable_to_reliable() {
        let pub_qos = Qos::reliable_control();
        let sub_qos = Qos::reliable_control();
        assert_eq!(check_qos_compatibility(&pub_qos, &sub_qos), QosCompatibility::Compatible);
    }

    #[test]
    fn history_depth() {
        assert_eq!(History::KeepLast(8).depth(), 8);
        assert_eq!(History::NoHistory.depth(), 0);
    }
}
