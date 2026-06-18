//! APEX Binary Serialization Protocol
//!
//! APEX uses its own wire format. Not protobuf. Not MessagePack. Not JSON.
//! The format is compact, deterministic, validatable, and versionable.
//!
//! Wire format per message:
//! ```text
//! +--------+----------+--------+---------+--------+-------+---------+-----------+
//! | magic  | version  | typeId | schemaV | schemaH| flags | payLen  | payload   |
//! | 2 B    | 1 B      | 4 B    | 2 B     | 8 B    | 1 B   | 4 B     | N bytes   |
//! +--------+----------+--------+---------+--------+-------+---------+-----------+
//! ```
//!
//! Optionally followed by a 4-byte CRC32 checksum if flag bit 0 is set.

pub mod codec;
pub mod header;
pub mod schema;
pub mod traits;

pub use codec::{ApexEncoder, ApexDecoder};
pub use header::{WireHeader, WireFlags};
pub use schema::{SchemaFingerprint, compute_schema_hash};
pub use traits::{ApexSerialize, ApexDeserialize};
