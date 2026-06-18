//! APEX Wire Header
//!
//! The wire header precedes every serialized APEX message.
//! It is fixed-size, alignment-friendly, and version-aware.
//! All multi-byte fields are little-endian.

/// Magic bytes that identify an APEX wire frame.
/// Chosen to be unlikely in other binary streams.
pub const APEX_MAGIC: [u8; 2] = [0xAE, 0x58]; // 0xAE = 0b10101110, 0x58 = 'X'

/// Current wire protocol version.
pub const WIRE_VERSION: u8 = 1;

/// Total size of the wire header in bytes.
pub const HEADER_SIZE: usize = 22;

/// Bit flags in the wire header flags byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WireFlags(pub u8);

impl WireFlags {
    /// No flags set.
    pub const NONE: WireFlags = WireFlags(0);
    /// Payload is followed by a 4-byte CRC32 checksum.
    pub const CHECKSUM: WireFlags = WireFlags(0b0000_0001);
    /// Payload is compressed (reserved, not yet active).
    pub const COMPRESSED: WireFlags = WireFlags(0b0000_0010);
    /// Message is part of a fragmented sequence.
    pub const FRAGMENTED: WireFlags = WireFlags(0b0000_0100);
    /// This is the last fragment in a sequence.
    pub const LAST_FRAGMENT: WireFlags = WireFlags(0b0000_1000);

    pub fn has_checksum(self) -> bool {
        (self.0 & Self::CHECKSUM.0) != 0
    }

    pub fn has_compression(self) -> bool {
        (self.0 & Self::COMPRESSED.0) != 0
    }

    pub fn is_fragmented(self) -> bool {
        (self.0 & Self::FRAGMENTED.0) != 0
    }

    pub fn is_last_fragment(self) -> bool {
        (self.0 & Self::LAST_FRAGMENT.0) != 0
    }

    pub fn set(self, flag: WireFlags) -> WireFlags {
        WireFlags(self.0 | flag.0)
    }
}

/// The APEX wire header.
///
/// Layout (22 bytes total, little-endian):
/// ```text
/// Offset  Size  Field
/// 0       2     magic (0xAE 0x58)
/// 2       1     wire_version
/// 3       4     type_id (u32)
/// 7       2     schema_version (u16)
/// 9       8     schema_hash (u64)
/// 17      1     flags (WireFlags)
/// 18      4     payload_length (u32)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireHeader {
    /// Identifies the message type. Registered in the schema registry.
    pub type_id: u32,
    /// Schema version of the message type at publish time.
    pub schema_version: u16,
    /// xxHash64 fingerprint of the schema at publish time.
    /// Used for schema mismatch detection.
    pub schema_hash: u64,
    /// Behavioral flags for this frame.
    pub flags: WireFlags,
    /// Byte length of the payload that follows this header.
    pub payload_length: u32,
}

impl WireHeader {
    /// Serialize this header into a fixed 22-byte array.
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..2].copy_from_slice(&APEX_MAGIC);
        buf[2] = WIRE_VERSION;
        buf[3..7].copy_from_slice(&self.type_id.to_le_bytes());
        buf[7..9].copy_from_slice(&self.schema_version.to_le_bytes());
        buf[9..17].copy_from_slice(&self.schema_hash.to_le_bytes());
        buf[17] = self.flags.0;
        buf[18..22].copy_from_slice(&self.payload_length.to_le_bytes());
        buf
    }

    /// Deserialize a header from a 22-byte slice.
    ///
    /// Returns `None` if the magic or version is invalid.
    /// Callers must treat `None` as a protocol violation.
    pub fn decode(buf: &[u8]) -> Option<WireHeader> {
        if buf.len() < HEADER_SIZE {
            return None;
        }
        if buf[0..2] != APEX_MAGIC {
            return None;
        }
        if buf[2] != WIRE_VERSION {
            // Future: version negotiation. For now, reject.
            return None;
        }

        let type_id = u32::from_le_bytes(buf[3..7].try_into().ok()?);
        let schema_version = u16::from_le_bytes(buf[7..9].try_into().ok()?);
        let schema_hash = u64::from_le_bytes(buf[9..17].try_into().ok()?);
        let flags = WireFlags(buf[17]);
        let payload_length = u32::from_le_bytes(buf[18..22].try_into().ok()?);

        Some(WireHeader { type_id, schema_version, schema_hash, flags, payload_length })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_header() {
        let h = WireHeader {
            type_id: 42,
            schema_version: 3,
            schema_hash: 0xDEADBEEFCAFEBABE,
            flags: WireFlags::CHECKSUM,
            payload_length: 256,
        };
        let encoded = h.encode();
        assert_eq!(encoded.len(), HEADER_SIZE);
        let decoded = WireHeader::decode(&encoded);
        assert!(decoded.is_some());
        let decoded = decoded.unwrap();
        assert_eq!(decoded.type_id, 42);
        assert_eq!(decoded.schema_version, 3);
        assert_eq!(decoded.schema_hash, 0xDEADBEEFCAFEBABE);
        assert!(decoded.flags.has_checksum());
        assert_eq!(decoded.payload_length, 256);
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0] = 0xFF;
        buf[1] = 0xFF;
        assert_eq!(WireHeader::decode(&buf), None);
    }

    #[test]
    fn decode_rejects_wrong_version() {
        let mut buf = [0u8; HEADER_SIZE];
        buf[0..2].copy_from_slice(&APEX_MAGIC);
        buf[2] = 99; // wrong version
        assert_eq!(WireHeader::decode(&buf), None);
    }

    #[test]
    fn wire_flags_composition() {
        let f = WireFlags::NONE
            .set(WireFlags::CHECKSUM)
            .set(WireFlags::FRAGMENTED);
        assert!(f.has_checksum());
        assert!(f.is_fragmented());
        assert!(!f.has_compression());
    }
}
