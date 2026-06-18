//! Metric primitive types for `apex-observability`.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::MAX_LABEL_LEN;

// ---------------------------------------------------------------------------
// Label
// ---------------------------------------------------------------------------

/// A key-value label attached to a metric.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label {
  pub key: Arc<str>,
  pub value: Arc<str>,
}

impl Label {
  /// Create a new label, returning `None` if either string exceeds
  /// [`MAX_LABEL_LEN`] bytes.
  pub fn new(key: &str, value: &str) -> Option<Self> {
    if key.len() > MAX_LABEL_LEN || value.len() > MAX_LABEL_LEN {
      return None;
    }
    Some(Self {
      key: key.into(),
      value: value.into(),
    })
  }
}

// ---------------------------------------------------------------------------
// Counter
// ---------------------------------------------------------------------------

/// A monotonically increasing unsigned 64-bit counter.
///
/// Safe to clone and share across threads — all clones point to the same
/// underlying atomic.
#[derive(Debug, Clone)]
pub struct Counter {
  inner: Arc<AtomicU64>,
  pub labels: Arc<[Label]>,
}

impl Counter {
  /// Create a new counter initialised to zero.
  pub fn new(labels: Vec<Label>) -> Self {
    Self {
      inner: Arc::new(AtomicU64::new(0)),
      labels: labels.into(),
    }
  }

  /// Increment the counter by `delta`.
  #[inline]
  pub fn inc_by(&self, delta: u64) {
    self.inner.fetch_add(delta, Ordering::Relaxed);
  }

  /// Increment the counter by one.
  #[inline]
  pub fn inc(&self) {
    self.inc_by(1);
  }

  /// Read the current value.
  #[inline]
  pub fn get(&self) -> u64 {
    self.inner.load(Ordering::Relaxed)
  }
}

// ---------------------------------------------------------------------------
// Gauge
// ---------------------------------------------------------------------------

/// A signed 64-bit gauge that can freely increase or decrease.
#[derive(Debug, Clone)]
pub struct Gauge {
  inner: Arc<AtomicI64>,
  pub labels: Arc<[Label]>,
}

impl Gauge {
  /// Create a new gauge initialised to zero.
  pub fn new(labels: Vec<Label>) -> Self {
    Self {
      inner: Arc::new(AtomicI64::new(0)),
      labels: labels.into(),
    }
  }

  /// Set the gauge to an absolute value.
  #[inline]
  pub fn set(&self, value: i64) {
    self.inner.store(value, Ordering::Relaxed);
  }

  /// Add `delta` to the current value.
  #[inline]
  pub fn add(&self, delta: i64) {
    self.inner.fetch_add(delta, Ordering::Relaxed);
  }

  /// Subtract `delta` from the current value.
  #[inline]
  pub fn sub(&self, delta: i64) {
    self.inner.fetch_sub(delta, Ordering::Relaxed);
  }

  /// Read the current value.
  #[inline]
  pub fn get(&self) -> i64 {
    self.inner.load(Ordering::Relaxed)
  }
}

// ---------------------------------------------------------------------------
// Histogram
// ---------------------------------------------------------------------------

/// A histogram that accumulates observations into configurable buckets.
///
/// Bucket boundaries must be strictly increasing (validated at construction).
#[derive(Debug, Clone)]
pub struct Histogram {
  inner: Arc<Mutex<HistogramInner>>,
  pub labels: Arc<[Label]>,
}

#[derive(Debug)]
struct HistogramInner {
  /// Upper-bound of each bucket (last entry is +Inf implicitly).
  bounds: Box<[f64]>,
  /// Count of observations falling into each bucket (cumulative).
  counts: Box<[u64]>,
  sum: f64,
  count: u64,
}

impl Histogram {
  /// Create a histogram with the given bucket upper-bounds.
  ///
  /// Returns `None` if `bounds` is empty or not strictly increasing.
  pub fn new(bounds: Vec<f64>, labels: Vec<Label>) -> Option<Self> {
    if bounds.is_empty() {
      return None;
    }
    for w in bounds.windows(2) {
      if w[0] >= w[1] {
        return None;
      }
    }
    let len = bounds.len();
    Some(Self {
      inner: Arc::new(Mutex::new(HistogramInner {
        bounds: bounds.into(),
        counts: vec![0u64; len].into(),
        sum: 0.0,
        count: 0,
      })),
      labels: labels.into(),
    })
  }

  /// Record a single observation.
  pub fn observe(&self, value: f64) {
    let mut g = self.inner.lock().unwrap();
    g.sum += value;
    g.count += 1;
    for (i, &bound) in g.bounds.iter().enumerate() {
      if value <= bound {
        g.counts[i] += 1;
      }
    }
  }

  /// Take a snapshot of the current state.
  pub fn snapshot(&self) -> HistogramSnapshot {
    let g = self.inner.lock().unwrap();
    HistogramSnapshot {
      bounds: g.bounds.clone(),
      counts: g.counts.clone(),
      sum: g.sum,
      count: g.count,
    }
  }
}

/// Immutable snapshot of a [`Histogram`] at a point in time.
#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
  pub bounds: Box<[f64]>,
  pub counts: Box<[u64]>,
  pub sum: f64,
  pub count: u64,
}
