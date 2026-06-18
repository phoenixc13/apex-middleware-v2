//! Global metric registry for `apex-observability`.
//!
//! The registry is the single source of truth for all named metrics.
//! It is bounded by [`crate::MAX_METRICS`] entries and is safe to
//! access from multiple threads via its internal `RwLock`.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::{
  error::ObsError,
  metrics::{Counter, Gauge, Histogram, Label},
  MAX_METRICS, MAX_NAME_LEN,
};

// ---------------------------------------------------------------------------
// MetricEntry
// ---------------------------------------------------------------------------

/// A single registered metric, discriminated by kind.
#[derive(Debug, Clone)]
pub enum MetricEntry {
  Counter(Counter),
  Gauge(Gauge),
  Histogram(Histogram),
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Thread-safe, bounded metric registry.
#[derive(Debug, Default)]
pub struct Registry {
  inner: RwLock<HashMap<Arc<str>, MetricEntry>>,
}

impl Registry {
  /// Create an empty registry.
  pub fn new() -> Self {
    Self::default()
  }

  // -- registration helpers --------------------------------------------------

  fn validate_name(name: &str) -> Result<(), ObsError> {
    if name.len() > MAX_NAME_LEN {
      return Err(ObsError::NameTooLong { len: name.len() });
    }
    Ok(())
  }

  fn check_capacity(map: &HashMap<Arc<str>, MetricEntry>) -> Result<(), ObsError> {
    if map.len() >= MAX_METRICS {
      return Err(ObsError::RegistryFull);
    }
    Ok(())
  }

  /// Register a [`Counter`].  Returns the counter handle on success,
  /// or an error if the registry is full, the name is too long, or a
  /// metric with that name already exists.
  pub fn register_counter(
    &self,
    name: &str,
    labels: Vec<Label>,
  ) -> Result<Counter, ObsError> {
    Self::validate_name(name)?;
    let mut map = self.inner.write().unwrap();
    if map.contains_key(name) {
      return Err(ObsError::DuplicateMetric {
        name: Box::leak(name.to_owned().into_boxed_str()),
      });
    }
    Self::check_capacity(&map)?;
    let counter = Counter::new(labels);
    map.insert(Arc::from(name), MetricEntry::Counter(counter.clone()));
    Ok(counter)
  }

  /// Register a [`Gauge`].
  pub fn register_gauge(
    &self,
    name: &str,
    labels: Vec<Label>,
  ) -> Result<Gauge, ObsError> {
    Self::validate_name(name)?;
    let mut map = self.inner.write().unwrap();
    if map.contains_key(name) {
      return Err(ObsError::DuplicateMetric {
        name: Box::leak(name.to_owned().into_boxed_str()),
      });
    }
    Self::check_capacity(&map)?;
    let gauge = Gauge::new(labels);
    map.insert(Arc::from(name), MetricEntry::Gauge(gauge.clone()));
    Ok(gauge)
  }

  /// Register a [`Histogram`].
  pub fn register_histogram(
    &self,
    name: &str,
    bounds: Vec<f64>,
    labels: Vec<Label>,
  ) -> Result<Histogram, ObsError> {
    Self::validate_name(name)?;
    let mut map = self.inner.write().unwrap();
    if map.contains_key(name) {
      return Err(ObsError::DuplicateMetric {
        name: Box::leak(name.to_owned().into_boxed_str()),
      });
    }
    Self::check_capacity(&map)?;
    let histogram =
      Histogram::new(bounds, labels).ok_or(ObsError::InvalidBuckets)?;
    map.insert(Arc::from(name), MetricEntry::Histogram(histogram.clone()));
    Ok(histogram)
  }

  // -- query -----------------------------------------------------------------

  /// Look up a metric by name.
  pub fn get(&self, name: &str) -> Option<MetricEntry> {
    self.inner.read().unwrap().get(name).cloned()
  }

  /// Number of registered metrics.
  pub fn len(&self) -> usize {
    self.inner.read().unwrap().len()
  }

  /// Returns `true` when no metrics are registered.
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Iterate over a snapshot of all registered metrics.
  pub fn snapshot(&self) -> Vec<(Arc<str>, MetricEntry)> {
    self.inner
      .read()
      .unwrap()
      .iter()
      .map(|(k, v)| (k.clone(), v.clone()))
      .collect()
  }
}

// ---------------------------------------------------------------------------
// Global singleton
// ---------------------------------------------------------------------------

static GLOBAL_REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Return a reference to the process-wide metric registry.
///
/// The registry is lazily initialised on first call.
pub fn global() -> &'static Registry {
  GLOBAL_REGISTRY.get_or_init(Registry::new)
}
