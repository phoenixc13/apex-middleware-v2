//! Length-prefix framing codec for TCP streams.
//!
//! ## Wire Format
//! ```text
//! [u32 frame_len_le (4 bytes)] [APEX FrameHeader (16 bytes)] [payload (frame_len - 16 bytes)]
//! ```

use std::io::{Read, Write};

use crate::{
  error::TcpError,
  MAX_TCP_PAYLOAD,
};

/// Size of the length prefix in bytes.
const LEN_PREFIX: usize = 4;

/// Shared header size from the UDP frame module re-used here.
const FRAME_HEADER_SIZE: usize = 16;

/// Encode an APEX frame (header + payload) with a 4-byte length prefix
/// and write it to `writer`.
///
/// `header_bytes` must be exactly 16 bytes.
pub fn encode_frame<W: Write>(
  writer: &mut W,
  header_bytes: &[u8; 16],
  payload: &[u8],
) -> Result<(), TcpError> {
  if payload.len() > MAX_TCP_PAYLOAD {
    return Err(TcpError::PayloadTooLarge { len: payload.len() });
  }
  let frame_len = (FRAME_HEADER_SIZE + payload.len()) as u32;
  let prefix = frame_len.to_le_bytes();
  writer.write_all(&prefix).map_err(|e| TcpError::Write { source: e })?;
  writer.write_all(header_bytes).map_err(|e| TcpError::Write { source: e })?;
  writer.write_all(payload).map_err(|e| TcpError::Write { source: e })?;
  Ok(())
}

/// Read exactly `n` bytes from `reader` into `buf`.
/// Returns `TcpError::ConnectionClosed` on EOF.
fn read_exact<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<(), TcpError> {
  let mut pos = 0;
  while pos < buf.len() {
    let n = reader.read(&mut buf[pos..]).map_err(|e| TcpError::Read { source: e })?;
    if n == 0 {
      return Err(TcpError::ConnectionClosed);
    }
    pos += n;
  }
  Ok(())
}

/// Decode one APEX frame from `reader`.
///
/// Returns `(header_bytes, payload)` on success.
/// Returns [`TcpError::ConnectionClosed`] on clean EOF.
/// Returns [`TcpError::FrameTooLarge`] if the frame exceeds the configured maximum.
pub fn decode_frame<R: Read>(
  reader: &mut R,
) -> Result<([u8; 16], Vec<u8>), TcpError> {
  // Read 4-byte length prefix.
  let mut prefix = [0u8; LEN_PREFIX];
  read_exact(reader, &mut prefix)?;
  let frame_len = u32::from_le_bytes(prefix);
  if frame_len as usize > FRAME_HEADER_SIZE + MAX_TCP_PAYLOAD {
    return Err(TcpError::FrameTooLarge { len: frame_len });
  }
  if (frame_len as usize) < FRAME_HEADER_SIZE {
    // Frame too short to contain a valid header.
    return Err(TcpError::FrameTooLarge { len: frame_len });
  }
  // Read header.
  let mut header_bytes = [0u8; 16];
  read_exact(reader, &mut header_bytes)?;
  // Read payload.
  let payload_len = frame_len as usize - FRAME_HEADER_SIZE;
  let mut payload = vec![0u8; payload_len];
  if payload_len > 0 {
    read_exact(reader, &mut payload)?;
  }
  Ok((header_bytes, payload))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::Cursor;

  #[test]
  fn encode_decode_round_trip() {
    let header = [0xAA_u8; 16];
    let payload = b"hello, APEX";
    let mut buf = Vec::new();
    encode_frame(&mut buf, &header, payload).unwrap();
    let mut cursor = Cursor::new(buf);
    let (decoded_hdr, decoded_payload) = decode_frame(&mut cursor).unwrap();
    assert_eq!(decoded_hdr, header);
    assert_eq!(decoded_payload, payload);
  }

  #[test]
  fn empty_payload_round_trip() {
    let header = [0x00_u8; 16];
    let payload: &[u8] = &[];
    let mut buf = Vec::new();
    encode_frame(&mut buf, &header, payload).unwrap();
    let mut cursor = Cursor::new(buf);
    let (_, decoded_payload) = decode_frame(&mut cursor).unwrap();
    assert!(decoded_payload.is_empty());
  }
}
