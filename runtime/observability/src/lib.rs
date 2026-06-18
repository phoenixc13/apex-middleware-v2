//! # apex-observability
//!
//! Structured metrics, counters, and event tracing for the APEX runtime.
//! Designed as a zero-allocation hot-path layer: all metric updates are
//! lock-free atomic operations. Reporting (exporting to Prometheus, JSON,
//! or structured log sinks) is handled out-of-band.
//!
//! ## Design Principles
//! 1. **No external runtime dependency.** Works in `no_std` + alloc.
//! 2. **Zero silent failures.** Every metric has an explicit name and label.
//! 3. **Bounded storage.** The global registry has a fixed capacity; attempting
//!    to register beyond capacity returns `ObsError::RegistryFull`.
//! 4. **Pull-based export.** Metrics are collected by calling `snapshot()`;
//!    the runtime itself does not spawn background threads.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

pub mod counter;
pub mod gauge;
pub mod histogram;
pub mod registry;
pub mod event;
pub mod error;

pub use counter::Counter;
pub use gauge::Gauge;
pub use histogram::Histogram;
pub use registry::{MetricRegistry, Snapshot};
pub use event::{Event, EventLevel, EventSink};
pub use error::ObsError;

/// Maximum number of metrics that can be registered.
pub const MAX_METRICS: usize = 1_024;

/// Maximum label string length (bytes).
pub const MAX_LABEL_LEN: usize = 64;

/// Maximum metric name length (bytes).
pub const MAX_NAME_LEN: usize = 64;
