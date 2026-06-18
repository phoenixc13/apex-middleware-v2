//! High-level SHM channel API: [`ShmPublisher`] and [`ShmSubscriber`].
//!
//! Both types are `Send` but not `Sync`; concurrent access from multiple
//! threads on the same side must go through distinct instances.

use std::{
  sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
  },
  thread,
  time::Duration,
};

use crate::{
  error::ShmError,
  ring::{RingHeader, Slot, LAYOUT_VERSION, MAGIC, SEGMENT_SIZE},
  MAX_SLOT_PAYLOAD, RING_SLOTS,
};

// ---------------------------------------------------------------------------
// Internal shared segment handle
// ---------------------------------------------------------------------------

/// Owning handle to a mapped SHM segment.
/// Dropped when the last Arc reference disappears.
struct ShmSegment {
  /// Raw pointer to the start of the mapped region.
  ptr: *mut u8,
  /// Total mapped size in bytes.
  size: usize,
  /// POSIX name (used for cleanup).
  name: String,
  /// True if this process created the segment and must unlink it.
  owner: bool,
}

// SAFETY: The raw pointer is only accessed through &self or &mut self
// references with proper atomic synchronisation.
unsafe impl Send for ShmSegment {}

impl ShmSegment {
  fn header(&self) -> &RingHeader {
    // SAFETY: ptr is valid for `size` bytes and aligned per mmap guarantees.
    unsafe { &*(self.ptr as *const RingHeader) }
  }

  fn header_mut(&mut self) -> &mut RingHeader {
    unsafe { &mut *(self.ptr as *mut RingHeader) }
  }

  fn slot(&self, index: usize) -> &Slot {
    debug_assert!(index < RING_SLOTS);
    let offset = std::mem::size_of::<RingHeader>() + index * std::mem::size_of::<Slot>();
    // SAFETY: offset is within the mapped region by construction.
    unsafe { &*(self.ptr.add(offset) as *const Slot) }
  }

  fn slot_mut(&mut self, index: usize) -> &mut Slot {
    debug_assert!(index < RING_SLOTS);
    let offset = std::mem::size_of::<RingHeader>() + index * std::mem::size_of::<Slot>();
    unsafe { &mut *(self.ptr.add(offset) as *mut Slot) }
  }
}

impl Drop for ShmSegment {
  fn drop(&mut self) {
    // Unmap the region.
    // SAFETY: ptr and size were obtained from a successful mmap call.
    #[cfg(unix)]
    unsafe {
      libc::munmap(self.ptr as *mut libc::c_void, self.size);
      if self.owner {
        let c_name = std::ffi::CString::new(self.name.as_str()).unwrap();
        libc::shm_unlink(c_name.as_ptr());
      }
    }
  }
}

// ---------------------------------------------------------------------------
// ShmPublisher
// ---------------------------------------------------------------------------

/// Zero-copy SHM publisher for a single topic channel.
///
/// The publisher creates the SHM segment on construction and owns its
/// lifecycle. When dropped, it unlinks the segment.
pub struct ShmPublisher {
  seg: ShmSegment,
}

impl ShmPublisher {
  /// Open (or create) a SHM channel for `topic`.
  ///
  /// # Errors
  /// Returns [`ShmError`] if the topic is too long, SHM creation fails,
  /// or memory mapping fails.
  #[cfg(unix)]
  pub fn open(topic: &str) -> Result<Self, ShmError> {
    if topic.len() > crate::MAX_TOPIC_LEN {
      return Err(ShmError::TopicNameTooLong { len: topic.len() });
    }
    let shm_name = shm_name_for_topic(topic);
    let seg = create_segment(&shm_name, true)?;
    // Initialise the header.
    let ptr = seg.ptr as *mut RingHeader;
    // SAFETY: segment is freshly created and exclusively owned.
    unsafe { RingHeader::init(ptr) };
    // Zero all slot sequence numbers.
    for i in 0..RING_SLOTS {
      let slot_offset =
        std::mem::size_of::<RingHeader>() + i * std::mem::size_of::<Slot>();
      let seq_ptr = unsafe {
        seg.ptr.add(slot_offset) as *mut AtomicU64
      };
      unsafe { (*seq_ptr).store(u64::MAX, Ordering::SeqCst) };
    }
    Ok(Self { seg })
  }

  /// Write `payload` into the next ring buffer slot.
  ///
  /// Returns [`ShmError::PayloadTooLarge`] if `payload` exceeds
  /// [`MAX_SLOT_PAYLOAD`]. Returns [`ShmError::RingFull`] if the
  /// slowest subscriber is still occupying the target slot.
  pub fn publish(&mut self, payload: &[u8]) -> Result<(), ShmError> {
    if payload.len() > MAX_SLOT_PAYLOAD {
      return Err(ShmError::PayloadTooLarge { len: payload.len() });
    }
    let header = self.seg.header();
    let seq = header.write_seq.load(Ordering::Relaxed);
    let next_seq = seq.wrapping_add(1);
    let slot_idx = (next_seq as usize) % RING_SLOTS;
    let slot = self.seg.slot_mut(slot_idx);
    // Check the slot is free (sequence == u64::MAX sentinel or stale read).
    let slot_seq = slot.header.sequence.load(Ordering::Acquire);
    if slot_seq != u64::MAX && slot_seq.wrapping_add(RING_SLOTS as u64) > next_seq {
      return Err(ShmError::RingFull);
    }
    // Write payload.
    slot.payload[..payload.len()].copy_from_slice(payload);
    slot.header.payload_len = payload.len() as u32;
    // Release store: payload must be visible before sequence update.
    slot.header.sequence.store(next_seq, Ordering::Release);
    // Advance the global write cursor.
    self.seg.header_mut().write_seq.store(next_seq, Ordering::Release);
    Ok(())
  }
}

// ---------------------------------------------------------------------------
// ShmSubscriber
// ---------------------------------------------------------------------------

/// Zero-copy SHM subscriber for a single topic channel.
///
/// The subscriber opens an existing segment created by a [`ShmPublisher`].
/// It does not own the segment lifetime.
pub struct ShmSubscriber {
  seg: ShmSegment,
  /// Next expected sequence number.
  read_seq: u64,
}

impl ShmSubscriber {
  /// Attach to an existing SHM channel for `topic`.
  ///
  /// # Errors
  /// Returns [`ShmError`] if the segment does not exist, mapping fails,
  /// or the header layout is incompatible.
  #[cfg(unix)]
  pub fn open(topic: &str) -> Result<Self, ShmError> {
    if topic.len() > crate::MAX_TOPIC_LEN {
      return Err(ShmError::TopicNameTooLong { len: topic.len() });
    }
    let shm_name = shm_name_for_topic(topic);
    let seg = open_segment(&shm_name)?;
    seg.header().validate()?;
    let read_seq = seg.header().write_seq.load(Ordering::Acquire);
    Ok(Self { seg, read_seq })
  }

  /// Non-blocking receive. Returns a copy of the next available payload.
  ///
  /// Returns [`ShmError::NoData`] when no new message is available.
  pub fn try_recv(&mut self) -> Result<Vec<u8>, ShmError> {
    let next = self.read_seq.wrapping_add(1);
    let slot_idx = (next as usize) % RING_SLOTS;
    let slot = self.seg.slot(slot_idx);
    let seq = slot.header.sequence.load(Ordering::Acquire);
    if seq != next {
      return Err(ShmError::NoData);
    }
    let len = slot.header.payload_len as usize;
    let data = slot.payload[..len].to_vec();
    self.read_seq = next;
    Ok(data)
  }

  /// Blocking receive with exponential back-off then OS yield.
  ///
  /// Back-off: 1 µs → 2 µs → 4 µs → ... → 1 ms (plateau).
  pub fn recv(&mut self) -> Result<Vec<u8>, ShmError> {
    let mut delay = Duration::from_micros(1);
    loop {
      match self.try_recv() {
        Ok(data) => return Ok(data),
        Err(ShmError::NoData) => {
          thread::sleep(delay);
          delay = (delay * 2).min(Duration::from_millis(1));
        }
        Err(e) => return Err(e),
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Internal helpers (Unix-only)
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn shm_name_for_topic(topic: &str) -> String {
  // Derive a stable POSIX name from the topic string.
  // Max 31 chars on some kernels; use a hash.
  use std::hash::{Hash, Hasher};
  use std::collections::hash_map::DefaultHasher;
  let mut h = DefaultHasher::new();
  topic.hash(&mut h);
  format!("/apex_{:016x}", h.finish())
}

#[cfg(unix)]
fn create_segment(name: &str, owner: bool) -> Result<ShmSegment, ShmError> {
  use std::ffi::CString;
  let c_name = CString::new(name).unwrap();
  // SAFETY: FFI call with valid null-terminated string.
  let fd = unsafe {
    libc::shm_open(
      c_name.as_ptr(),
      libc::O_CREAT | libc::O_RDWR | libc::O_TRUNC,
      0o600,
    )
  };
  if fd < 0 {
    return Err(ShmError::ShmOpen {
      name: name.to_owned(),
      source: std::io::Error::last_os_error(),
    });
  }
  if unsafe { libc::ftruncate(fd, SEGMENT_SIZE as libc::off_t) } != 0 {
    unsafe { libc::close(fd) };
    return Err(ShmError::Truncate {
      source: std::io::Error::last_os_error(),
    });
  }
  map_segment(fd, name, owner)
}

#[cfg(unix)]
fn open_segment(name: &str) -> Result<ShmSegment, ShmError> {
  use std::ffi::CString;
  let c_name = CString::new(name).unwrap();
  let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0) };
  if fd < 0 {
    return Err(ShmError::ShmOpen {
      name: name.to_owned(),
      source: std::io::Error::last_os_error(),
    });
  }
  map_segment(fd, name, false)
}

#[cfg(unix)]
fn map_segment(fd: i32, name: &str, owner: bool) -> Result<ShmSegment, ShmError> {
  let ptr = unsafe {
    libc::mmap(
      std::ptr::null_mut(),
      SEGMENT_SIZE,
      libc::PROT_READ | libc::PROT_WRITE,
      libc::MAP_SHARED,
      fd,
      0,
    )
  };
  unsafe { libc::close(fd) };
  if ptr == libc::MAP_FAILED {
    return Err(ShmError::Mmap {
      source: std::io::Error::last_os_error(),
    });
  }
  Ok(ShmSegment {
    ptr: ptr as *mut u8,
    size: SEGMENT_SIZE,
    name: name.to_owned(),
    owner,
  })
}
