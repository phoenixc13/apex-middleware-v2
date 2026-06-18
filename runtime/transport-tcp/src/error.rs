//! Error types for `apex-transport-tcp`.

use std::{fmt, io};

/// All errors from the TCP transport layer.
#[derive(Debug)]
pub enum TcpError {
  /// Payload exceeds [`crate::MAX_TCP_PAYLOAD`].
  PayloadTooLarge { len: usize },
  /// Socket bind failed.
  Bind { addr: String, source: io::Error },
  /// `connect()` failed.
  Connect { addr: String, source: io::Error },
  /// `accept()` failed.
  Accept { source: io::Error },
  /// Write to stream failed.
  Write { source: io::Error },
  /// Read from stream failed.
  Read { source: io::Error },
  /// The remote peer closed the connection cleanly.
  ConnectionClosed,
  /// Frame length prefix exceeds the configured maximum.
  FrameTooLarge { len: u32 },
  /// Set socket option failed.
  SockOpt { opt: &'static str, source: io::Error },
  /// An OS error not covered above.
  Os { source: io::Error },
}

impl fmt::Display for TcpError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::PayloadTooLarge { len } =>
        write!(f, "payload too large: {} bytes (max {})", len, crate::MAX_TCP_PAYLOAD),
      Self::Bind { addr, source } =>
        write!(f, "bind to '{}' failed: {}", addr, source),
      Self::Connect { addr, source } =>
        write!(f, "connect to '{}' failed: {}", addr, source),
      Self::Accept { source } =>
        write!(f, "accept failed: {}", source),
      Self::Write { source } =>
        write!(f, "TCP write failed: {}", source),
      Self::Read { source } =>
        write!(f, "TCP read failed: {}", source),
      Self::ConnectionClosed =>
        write!(f, "remote peer closed connection"),
      Self::FrameTooLarge { len } =>
        write!(f, "frame too large: {} bytes", len),
      Self::SockOpt { opt, source } =>
        write!(f, "setsockopt({}) failed: {}", opt, source),
      Self::Os { source } =>
        write!(f, "OS error: {}", source),
    }
  }
}

impl std::error::Error for TcpError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::Bind { source, .. }
      | Self::Connect { source, .. }
      | Self::Accept { source }
      | Self::Write { source }
      | Self::Read { source }
      | Self::SockOpt { source, .. }
      | Self::Os { source } => Some(source),
      _ => None,
    }
  }
}
