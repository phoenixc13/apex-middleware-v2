//! Error types for `apex-observability`.

use std::fmt;

/// All errors from the observability layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObsError {
  /// The global metric registry has no remaining capacity.
  RegistryFull,
  /// The metric name exceeds [`crate::MAX_NAME_LEN`] bytes.
  NameTooLong { len: usize },
  /// A label exceeds [`crate::MAX_LABEL_LEN`] bytes.
  LabelTooLong { len: usize },
  /// A metric with this name already exists.
  DuplicateMetric { name: &'static str },
  /// Histogram bucket boundaries are not monotonically increasing.
  InvalidBuckets,
}

impl fmt::Display for ObsError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::RegistryFull =>
        write!(f, "metric registry full (max {} entries)", crate::MAX_METRICS),
      Self::NameTooLong { len } =>
        write!(f, "metric name too long: {} bytes (max {})", len, crate::MAX_NAME_LEN),
      Self::LabelTooLong { len } =>
        write!(f, "label too long: {} bytes (max {})", len, crate::MAX_LABEL_LEN),
      Self::DuplicateMetric { name } =>
        write!(f, "metric '{}' already registered", name),
      Self::InvalidBuckets =>
        write!(f, "histogram bucket boundaries must be strictly increasing"),
    }
  }
}

impl std::error::Error for ObsError {}
