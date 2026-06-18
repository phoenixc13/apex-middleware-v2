//! APEX UDP wire frame: a fixed 16-byte header prepended to every datagram.
//!
//! ## Layout (little-endian)
//! ```text
//! Offset  Size  Field
//! 0       4     magic        (0xA9EE_0002)
//! 4       1     version      (0x01)
//! 5       1     flags        (reserved, must be 0)
//! 6       2     payload_len  (u16, max 1472)
//! 8       8     topic_hash   (u64, FNV-1a of topic name)
//! ```
//! Total: 16 bytes.

use std::mem;

/// Wire magic for UDP frames.
pub const UDP_MAGIC: u32 = 0xA9EE_0002;
/// Wire format version.
pub const UDP_VERSION: u8 = 1;
/// Frame header size in bytes.
pub const HEADER_SIZE: usize = 16;

/// Fixed-size UDP frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct FrameHeader {
  /// Magic number.
  pub magic: u32,
  /// Format version.
  pub version: u8,
  /// Reserved flags (must be 0).
  pub flags: u8,
  /// Length of the payload that follows (bytes).
  pub payload_len: u16,
  /// FNV-1a hash of the topic name (8 bytes).
  pub topic_hash: u64,
}

const _: () = assert!(mem::size_of::<FrameHeader>() == HEADER_SIZE, "FrameHeader size mismatch");

impl FrameHeader {
  /// Construct a valid header for the given topic hash and payload length.
  pub fn new(topic_hash: u64, payload_len: u16) -> Self {
    Self {
      magic: UDP_MAGIC.to_le(),
      version: UDP_VERSION,
      flags: 0,
      payload_len: payload_len.to_le(),
      topic_hash: topic_hash.to_le(),
    }
  }

  /// Serialize to bytes.
  pub fn to_bytes(self) -> [u8; HEADER_SIZE] {
    // SAFETY: FrameHeader is repr(C, packed) with no padding and only
    // integer fields. Transmuting to bytes is well-defined.
    unsafe { mem::transmute(self) }
  }

  /// Deserialize from bytes. Returns `None` if magic or version is wrong.
  pub fn from_bytes(bytes: &[u8; HEADER_SIZE]) -> Option<Self> {
    // SAFETY: same justification as to_bytes.
    let h: Self = unsafe { mem::transmute(*bytes) };
    let magic = u32::from_le(h.magic);
    if magic != UDP_MAGIC || h.version != UDP_VERSION {
      return None;
    }
    Some(h)
  }

  /// Topic hash as host-endian u64.
  pub fn topic_hash_host(&self) -> u64 {
    u64::from_le(self.topic_hash)
  }

  /// Payload length as host-endian u16.
  pub fn payload_len_host(&self) -> u16 {
    u16::from_le(self.payload_len)
  }
}

/// Compute FNV-1a hash of a topic name byte slice.
pub fn topic_hash(topic: &str) -> u64 {
  const OFFSET: u64 = 14_695_981_039_346_656_037;
  const PRIME: u64 = 1_099_511_628_211;
  let mut h = OFFSET;
  for byte in topic.bytes() {
    h ^= byte as u64;
    h = h.wrapping_mul(PRIME);
  }
  h
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn header_round_trip() {
    let hash = topic_hash("/sensors/lidar");
    let hdr = FrameHeader::new(hash, 512);
    let bytes = hdr.to_bytes();
    let decoded = FrameHeader::from_bytes(&bytes).expect("valid header");
    assert_eq!(decoded.topic_hash_host(), hash);
    assert_eq!(decoded.payload_len_host(), 512);
  }

  #[test]
  fn bad_magic_rejected() {
    let mut bytes = FrameHeader::new(0, 0).to_bytes();
    bytes[0] = 0xFF;
    assert!(FrameHeader::from_bytes(&bytes).is_none());
  }
}
