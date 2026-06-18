//! # apex-transport-shm
//!
//! Shared Memory (SHM) zero-copy transport for intra-host communication.
//! Implements a lock-free ring buffer over POSIX shared memory segments.
//! No kernel crossings on the hot path after channel establishment.
//!
//! ## Architecture
//! - One SHM segment per topic channel (named by topic hash).
//! - Ring buffer header at offset 0; slots follow contiguously.
//! - Publisher writes slot index atomically; subscriber reads via futex-free spin
//!   with exponential back-off then OS yield.
//! - Segment lifecycle managed by the first publisher; cleaned up on last close.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod channel;
pub mod error;
pub mod ring;

pub use channel::{ShmPublisher, ShmSubscriber};
pub use error::ShmError;

/// Maximum topic name length (bytes, UTF-8).
pub const MAX_TOPIC_LEN: usize = 128;

/// Maximum payload size per slot (bytes).
pub const MAX_SLOT_PAYLOAD: usize = 65_536; // 64 KiB

/// Number of slots in the ring buffer.
/// Must be a power of two.
pub const RING_SLOTS: usize = 64;

const _: () = assert!(RING_SLOTS.is_power_of_two(), "RING_SLOTS must be a power of two");
