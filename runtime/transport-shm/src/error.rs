//! Error types for the SHM transport layer.

use std::fmt;

/// All errors that can originate from `apex-transport-shm`.
#[derive(Debug)]
pub enum ShmError {
  /// The topic name exceeds [`crate::MAX_TOPIC_LEN`] bytes.
  TopicNameTooLong { len: usize },
  /// Failed to open or create the POSIX shared memory object.
  ShmOpen {
    name: String,
    source: std::io::Error,
  },
  /// `ftruncate` failed when sizing the SHM segment.
  Truncate { source: std::io::Error },
  /// `mmap` returned an error.
  Mmap { source: std::io::Error },
  /// The segment already exists but has an incompatible layout version.
  LayoutMismatch { found: u32, expected: u32 },
  /// The ring buffer is full; the publisher must back off.
  RingFull,
  /// The subscriber has no new data (non-blocking read).
  NoData,
  /// The payload exceeds [`crate::MAX_SLOT_PAYLOAD`] bytes.
  PayloadTooLarge { len: usize },
  /// An underlying OS error not covered by the variants above.
  Os { source: std::io::Error },
}

impl fmt::Display for ShmError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::TopicNameTooLong { len } =>
        write!(f, "topic name too long: {} bytes (max {})", len, crate::MAX_TOPIC_LEN),
      Self::ShmOpen { name, source } =>
        write!(f, "shm_open failed for '{}': {}", name, source),
      Self::Truncate { source } =>
        write!(f, "ftruncate failed: {}", source),
      Self::Mmap { source } =>
        write!(f, "mmap failed: {}", source),
      Self::LayoutMismatch { found, expected } =>
        write!(f, "SHM layout version mismatch: found {}, expected {}", found, expected),
      Self::RingFull =>
        write!(f, "ring buffer full: publisher must back off"),
      Self::NoData =>
        write!(f, "no new data available (non-blocking)"),
      Self::PayloadTooLarge { len } =>
        write!(f, "payload too large: {} bytes (max {})", len, crate::MAX_SLOT_PAYLOAD),
      Self::Os { source } =>
        write!(f, "OS error: {}", source),
    }
  }
}

impl std::error::Error for ShmError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      Self::ShmOpen { source, .. }
      | Self::Truncate { source }
      | Self::Mmap { source }
      | Self::Os { source } => Some(source),
      _ => None,
    }
  }
}
