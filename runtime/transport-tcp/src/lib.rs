//! # apex-transport-tcp
//!
//! Reliable ordered TCP transport for inter-host APEX communication.
//! Designed for edge-cloud bridging and scenarios where guaranteed delivery
//! and ordering are required (e.g., configuration distribution, cloud telemetry).
//!
//! ## Wire Protocol
//! Each TCP stream carries a sequence of length-prefixed APEX frames:
//! ```text
//! [u32 frame_len_le] [FrameHeader (16 B)] [payload (frame_len - 16 B)]
//! ```
//! `frame_len` is the total byte count of the APEX frame (header + payload),
//! NOT including the 4-byte length prefix itself.
//!
//! ## Architecture
//! - One TCP connection per remote peer.
//! - All I/O is blocking with configurable read/write timeouts.
//! - The `TcpBridge` type handles listener accept loops in a dedicated thread.
//! - Payloads must not exceed [`MAX_TCP_PAYLOAD`].

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod codec;
pub mod connection;
pub mod error;
pub mod listener;

pub use codec::{decode_frame, encode_frame};
pub use connection::TcpConnection;
pub use error::TcpError;
pub use listener::TcpListener;

/// Maximum payload size per TCP frame (bytes).
/// Larger than UDP MTU since TCP handles fragmentation internally.
pub const MAX_TCP_PAYLOAD: usize = 1_048_576; // 1 MiB

/// Default read timeout for blocking TCP operations.
pub const DEFAULT_READ_TIMEOUT_MS: u64 = 5_000;

/// Default write timeout for blocking TCP operations.
pub const DEFAULT_WRITE_TIMEOUT_MS: u64 = 5_000;

/// TCP keepalive idle time in seconds.
pub const TCP_KEEPALIVE_IDLE_SECS: u64 = 10;
