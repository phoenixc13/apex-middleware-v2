//! # apex-transport-udp
//!
//! UDP/IP unicast and multicast transport for inter-host APEX communication.
//!
//! ## Design Decisions
//! - **No fragmentation at this layer.** The caller (pubsub engine) is
//!   responsible for keeping payloads within the configured MTU.
//! - **Bounded send/receive buffers.** All sockets use fixed kernel SO_SNDBUF /
//!   SO_RCVBUF sizes; overflow is an explicit error, not silent loss.
//! - **Non-blocking I/O.** All sockets are set O_NONBLOCK. Async integration
//!   is handled at the pubsub layer via polling.
//! - **No DDS.** Wire format is the APEX binary frame defined in
//!   `apex-serializer`, not RTPS or CDR.
//!
//! ## Supported Modes
//! | Mode            | Use-case                            |
//! |-----------------|-------------------------------------|
//! | Unicast         | Point-to-point peer messaging       |
//! | Multicast       | One-to-many topic fan-out           |

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod error;
pub mod frame;
pub mod socket;
pub mod multicast;

pub use error::UdpError;
pub use socket::{UdpReceiver, UdpSender};
pub use multicast::{MulticastReceiver, MulticastSender};

/// Maximum UDP payload size APEX will use.
/// Conservative Ethernet MTU minus IP (20) + UDP (8) headers = 1472 bytes.
/// Larger payloads must be rejected at the caller level.
pub const MAX_UDP_PAYLOAD: usize = 1_472;

/// Default TTL for multicast datagrams.
pub const DEFAULT_MULTICAST_TTL: u32 = 1; // LAN scope only

/// Default socket receive buffer size (bytes).
pub const SOCK_RCVBUF: usize = 4 * 1024 * 1024; // 4 MiB

/// Default socket send buffer size (bytes).
pub const SOCK_SNDBUF: usize = 1 * 1024 * 1024; // 1 MiB
