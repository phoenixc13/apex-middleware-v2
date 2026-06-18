//! Lock-free ring buffer layout mapped over a POSIX SHM segment.
//!
//! ## Memory Layout
//! ```text
//! [ RingHeader (128 bytes, cache-line aligned) ]
//! [ Slot 0 (SlotHeader + payload[MAX_SLOT_PAYLOAD]) ]
//! [ Slot 1 ... ]
//! [ Slot N-1 ]
//! ```
//!
//! ## Invariants
//! - `write_seq` is monotonically increasing; never wraps within a session.
//! - Slot index = `write_seq % RING_SLOTS`.
//! - A slot is ready to read when its `sequence` field equals `write_seq`.
//! - The reader tracks its own `read_seq` locally (not in SHM).

use std::{
  mem,
  sync::atomic::{AtomicU64, Ordering},
};

use crate::{MAX_SLOT_PAYLOAD, RING_SLOTS};

/// Magic number written into the header to detect corrupt segments.
pub const SHM_MAGIC: u32 = 0xA9_EX_0001_u32.to_be();
// Note: above uses `to_be()` for endian-safe detection; corrected constant:
pub const SHM_MAGIC_LE: u32 = 0x0001_EX_A9; // placeholder — real value below
/// Actual magic (little-endian storage).
pub const MAGIC: u32 = 0xA9EE_0001;

/// Layout version. Incremented on any breaking change.
pub const LAYOUT_VERSION: u32 = 1;

/// Total size of one SHM segment in bytes.
pub const SEGMENT_SIZE: usize =
  mem::size_of::<RingHeader>() + RING_SLOTS * mem::size_of::<Slot>();

/// Ring buffer header — lives at offset 0 of the SHM segment.
/// Must be exactly 128 bytes (two cache lines) to avoid false sharing.
#[repr(C, align(64))]
pub struct RingHeader {
  /// Magic number for integrity verification.
  pub magic: u32,
  /// Layout version.
  pub version: u32,
  /// Number of slots (must equal `RING_SLOTS` at runtime).
  pub slot_count: u32,
  /// Maximum payload bytes per slot (must equal `MAX_SLOT_PAYLOAD`).
  pub max_payload: u32,
  /// Monotonically increasing write sequence counter.
  /// Written by the publisher, read by subscribers.
  pub write_seq: AtomicU64,
  /// Reserved padding to reach 128 bytes.
  pub _pad: [u8; 104],
}

const _: () = assert!(
  mem::size_of::<RingHeader>() == 128,
  "RingHeader must be exactly 128 bytes"
);

/// Per-slot header.
#[repr(C)]
pub struct SlotHeader {
  /// Sequence number of the message written into this slot.
  /// Readers compare against their local `read_seq`.
  pub sequence: AtomicU64,
  /// Actual payload length in bytes.
  pub payload_len: u32,
  /// Reserved.
  pub _pad: [u8; 4],
}

/// One ring buffer slot: header + fixed-size payload area.
#[repr(C)]
pub struct Slot {
  pub header: SlotHeader,
  pub payload: [u8; MAX_SLOT_PAYLOAD],
}

impl RingHeader {
  /// Initialise a freshly mapped header in the publisher.
  ///
  /// # Safety
  /// Caller must ensure `ptr` points to at least `size_of::<RingHeader>()` bytes
  /// of writable, properly aligned memory.
  pub unsafe fn init(ptr: *mut Self) {
    unsafe {
      (*ptr).magic = MAGIC;
      (*ptr).version = LAYOUT_VERSION;
      (*ptr).slot_count = RING_SLOTS as u32;
      (*ptr).max_payload = MAX_SLOT_PAYLOAD as u32;
      // SeqCst store: visible to all future subscribers immediately.
      (*ptr).write_seq.store(0, Ordering::SeqCst);
      (*ptr)._pad = [0u8; 104];
    }
  }

  /// Validate an existing header (subscriber side).
  pub fn validate(&self) -> Result<(), crate::ShmError> {
    if self.magic != MAGIC {
      return Err(crate::ShmError::LayoutMismatch {
        found: self.magic,
        expected: MAGIC,
      });
    }
    if self.version != LAYOUT_VERSION {
      return Err(crate::ShmError::LayoutMismatch {
        found: self.version,
        expected: LAYOUT_VERSION,
      });
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ring_header_size() {
    assert_eq!(mem::size_of::<RingHeader>(), 128);
  }

  #[test]
  fn slot_header_layout() {
    // SlotHeader must be < MAX_SLOT_PAYLOAD so payload field dominates slot size.
    assert!(mem::size_of::<SlotHeader>() < MAX_SLOT_PAYLOAD);
  }
}
