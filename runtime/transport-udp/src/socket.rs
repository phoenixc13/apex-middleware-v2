//! Unicast UDP sender and receiver.

use std::{
  io::{self, ErrorKind},
  net::{SocketAddr, UdpSocket},
};

use crate::{
  error::UdpError,
  frame::{topic_hash, FrameHeader, HEADER_SIZE},
  MAX_UDP_PAYLOAD, SOCK_RCVBUF, SOCK_SNDBUF,
};

// ---------------------------------------------------------------------------
// UdpSender
// ---------------------------------------------------------------------------

/// Unicast UDP sender.
///
/// Maintains a connected non-blocking UDP socket to a fixed remote address.
pub struct UdpSender {
  socket: UdpSocket,
  topic_hash: u64,
  /// Scratch buffer: [header (16 B)] + [payload (up to MAX_UDP_PAYLOAD)].
  buf: Vec<u8>,
}

impl UdpSender {
  /// Create a sender that sends to `remote` on topic `topic`.
  ///
  /// Binds to `0.0.0.0:0` (OS-assigned ephemeral port).
  pub fn new(topic: &str, remote: SocketAddr) -> Result<Self, UdpError> {
    let local: SocketAddr = if remote.is_ipv6() {
      "[::]:0".parse().unwrap()
    } else {
      "0.0.0.0:0".parse().unwrap()
    };
    let socket = UdpSocket::bind(local).map_err(|e| UdpError::Bind {
      addr: local.to_string(),
      source: e,
    })?;
    socket.connect(remote).map_err(|e| UdpError::Bind {
      addr: remote.to_string(),
      source: e,
    })?;
    set_nonblocking(&socket)?;
    set_sock_buf(&socket, SOCK_SNDBUF, SOCK_RCVBUF)?;
    let mut buf = Vec::with_capacity(HEADER_SIZE + MAX_UDP_PAYLOAD);
    buf.resize(HEADER_SIZE, 0u8);
    Ok(Self {
      socket,
      topic_hash: topic_hash(topic),
      buf,
    })
  }

  /// Send `payload` to the configured remote.
  ///
  /// Returns [`UdpError::PayloadTooLarge`] if payload exceeds the MTU limit.
  /// Returns [`UdpError::WouldBlock`] if the socket buffer is temporarily full.
  pub fn send(&mut self, payload: &[u8]) -> Result<(), UdpError> {
    if payload.len() > MAX_UDP_PAYLOAD {
      return Err(UdpError::PayloadTooLarge { len: payload.len() });
    }
    let header = FrameHeader::new(self.topic_hash, payload.len() as u16);
    let header_bytes = header.to_bytes();
    self.buf.clear();
    self.buf.extend_from_slice(&header_bytes);
    self.buf.extend_from_slice(payload);
    match self.socket.send(&self.buf) {
      Ok(_) => Ok(()),
      Err(e) if e.kind() == ErrorKind::WouldBlock => Err(UdpError::WouldBlock),
      Err(e) => Err(UdpError::Send { source: e }),
    }
  }
}

// ---------------------------------------------------------------------------
// UdpReceiver
// ---------------------------------------------------------------------------

/// Unicast UDP receiver.
///
/// Binds to a local address and receives datagrams filtered by topic hash.
pub struct UdpReceiver {
  socket: UdpSocket,
  topic_hash: u64,
  buf: [u8; HEADER_SIZE + MAX_UDP_PAYLOAD],
}

impl UdpReceiver {
  /// Bind to `local` and accept datagrams for `topic`.
  pub fn bind(topic: &str, local: SocketAddr) -> Result<Self, UdpError> {
    let socket = UdpSocket::bind(local).map_err(|e| UdpError::Bind {
      addr: local.to_string(),
      source: e,
    })?;
    set_nonblocking(&socket)?;
    set_sock_buf(&socket, SOCK_SNDBUF, SOCK_RCVBUF)?;
    Ok(Self {
      socket,
      topic_hash: topic_hash(topic),
      buf: [0u8; HEADER_SIZE + MAX_UDP_PAYLOAD],
    })
  }

  /// Non-blocking receive. Returns a copy of the payload bytes.
  ///
  /// Silently discards datagrams with wrong magic/version or topic mismatch.
  /// Returns [`UdpError::WouldBlock`] when no datagram is waiting.
  pub fn try_recv(&mut self) -> Result<(Vec<u8>, SocketAddr), UdpError> {
    loop {
      let (n, from) = match self.socket.recv_from(&mut self.buf) {
        Ok(v) => v,
        Err(e) if e.kind() == ErrorKind::WouldBlock => return Err(UdpError::WouldBlock),
        Err(e) => return Err(UdpError::Recv { source: e }),
      };
      if n < HEADER_SIZE {
        continue; // Too short — discard.
      }
      let header_bytes: &[u8; HEADER_SIZE] =
        self.buf[..HEADER_SIZE].try_into().unwrap();
      let Some(hdr) = FrameHeader::from_bytes(header_bytes) else {
        continue; // Bad magic or version — discard.
      };
      if hdr.topic_hash_host() != self.topic_hash {
        continue; // Wrong topic — discard.
      }
      let payload_len = hdr.payload_len_host() as usize;
      if n < HEADER_SIZE + payload_len {
        continue; // Truncated — discard.
      }
      let payload = self.buf[HEADER_SIZE..HEADER_SIZE + payload_len].to_vec();
      return Ok((payload, from));
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn set_nonblocking(s: &UdpSocket) -> Result<(), UdpError> {
  s.set_nonblocking(true)
    .map_err(|e| UdpError::SockOpt { opt: "O_NONBLOCK", source: e })
}

fn set_sock_buf(s: &UdpSocket, sndbuf: usize, rcvbuf: usize) -> Result<(), UdpError> {
  // Best-effort; some OS caps these values silently.
  // We log the attempt and continue on error.
  #[cfg(unix)]
  {
    use std::os::unix::io::AsRawFd;
    let fd = s.as_raw_fd();
    unsafe {
      let val = sndbuf as libc::c_int;
      libc::setsockopt(
        fd, libc::SOL_SOCKET, libc::SO_SNDBUF,
        &val as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
      );
      let val = rcvbuf as libc::c_int;
      libc::setsockopt(
        fd, libc::SOL_SOCKET, libc::SO_RCVBUF,
        &val as *const _ as *const libc::c_void,
        std::mem::size_of::<libc::c_int>() as libc::socklen_t,
      );
    }
  }
  let _ = (sndbuf, rcvbuf);
  Ok(())
}
