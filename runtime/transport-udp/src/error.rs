//! Error types for `apex-transport-udp`.

use std::{fmt, io, net::AddrParseError};

/// All errors that can originate from the UDP transport.
#[derive(Debug)]
pub enum UdpError {
  /// Payload exceeds [`crate::MAX_UDP_PAYLOAD`].
  PayloadTooLarge { len: usize },
  /// Socket bind / connect operation failed.
  Bind { addr: String, source: io::Error },
  /// Send operation failed.
  Send { source: io::Error },
  /// Receive operation failed.
  Recv { source: io::Error },
  /// No data available (would-block on non-blocking socket).
  WouldBlock,
  /// Setting socket option failed.
  SockOpt { opt: &'static str, source: io::Error },
  /// Invalid socket address string.
  InvalidAddr { addr: String, source: AddrParseError },
  /// Multicast group join failed.
  JoinMulticast { group: String, source: io::Error },
  /// An OS error not covered by the above.
  Os { source: io::Error },
}

impl fmt::Display for UdpError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::PayloadTooLarge { len } =>
        write!(f, "payload too large: {} bytes (max {})", len, crate::MAX_UDP_PAYLOAD),
      Self::Bind { addr, source } =>
        write!(f, "bind to '{}' failed: {}", addr, source),
      Self::Send { source } =>
        write!(f, "UDP send failed: {}", source),
      Self::Recv { source } =>
        write!(f, "UDP recv failed: {}", source),
      Self::WouldBlock =>
        write!(f, "no data available (non-blocking)"),
      Self::SockOpt { opt, source } =>
        write!(f, "setsockopt({}) failed: {}", opt, source),
      Self::InvalidAddr { addr, source } =>
        write!(f, "invalid address '{}': {}", addr, source),
      Self::JoinMulticast { group, source } =>
        write!(f, "join multicast group '{}' failed: {}", group, source),
      Self::Os { source } =>
        write!(f, "OS error: {}", source),
    }
  }
}

impl std::error::Error for UdpError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Bind { source, .. }
      | Self::Send { source }
      | Self::Recv { source }
      | Self::SockOpt { source, .. }
      | Self::JoinMulticast { source, .. }
      | Self::Os { source } => Some(source),
      Self::InvalidAddr { source, .. } => Some(source),
      _ => None,
    }
  }
}
