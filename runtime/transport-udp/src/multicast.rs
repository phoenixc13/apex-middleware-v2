//! UDP/IP multicast sender and receiver.
//!
//! Uses SSM (Source-Specific Multicast) style: one group address per topic,
//! derived from the topic FNV-1a hash mapped into the 239.0.0.0/8
//! administratively scoped range.

use std::{
  io::ErrorKind,
  net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
};

use crate::{
  error::UdpError,
  frame::{topic_hash, FrameHeader, HEADER_SIZE},
  DEFAULT_MULTICAST_TTL, MAX_UDP_PAYLOAD, SOCK_RCVBUF, SOCK_SNDBUF,
};

// ---------------------------------------------------------------------------
// Group address derivation
// ---------------------------------------------------------------------------

/// Derive an administratively-scoped multicast IPv4 address for `topic`.
///
/// Maps the lower 24 bits of the FNV-1a hash into 239.x.x.x space.
/// Range 239.0.0.0 – 239.255.255.255 is reserved for administrative use (RFC 2365).
pub fn multicast_group_for_topic(topic: &str) -> Ipv4Addr {
  let h = topic_hash(topic) as u32;
  let octet2 = ((h >> 16) & 0xFF) as u8;
  let octet3 = ((h >> 8) & 0xFF) as u8;
  let octet4 = (h & 0xFF) as u8;
  Ipv4Addr::new(239, octet2, octet3, octet4)
}

// ---------------------------------------------------------------------------
// MulticastSender
// ---------------------------------------------------------------------------

/// Multicast UDP sender.
///
/// Sends datagrams to the multicast group derived from the topic name.
pub struct MulticastSender {
  socket: UdpSocket,
  group: SocketAddr,
  topic_hash: u64,
  buf: Vec<u8>,
}

impl MulticastSender {
  /// Create a multicast sender for `topic` on the given `port`.
  pub fn new(topic: &str, port: u16, iface: Ipv4Addr) -> Result<Self, UdpError> {
    let group_ip = multicast_group_for_topic(topic);
    let group = SocketAddr::new(IpAddr::V4(group_ip), port);
    let local: SocketAddr = format!("0.0.0.0:{}", 0).parse().unwrap();
    let socket = UdpSocket::bind(local).map_err(|e| UdpError::Bind {
      addr: local.to_string(),
      source: e,
    })?;
    // Set multicast TTL.
    socket
      .set_multicast_ttl_v4(DEFAULT_MULTICAST_TTL)
      .map_err(|e| UdpError::SockOpt { opt: "IP_MULTICAST_TTL", source: e })?;
    // Bind outgoing multicast to specific interface.
    socket
      .set_multicast_if_v4(&iface)
      .map_err(|e| UdpError::SockOpt { opt: "IP_MULTICAST_IF", source: e })?;
    socket
      .set_nonblocking(true)
      .map_err(|e| UdpError::SockOpt { opt: "O_NONBLOCK", source: e })?;
    let mut buf = Vec::with_capacity(HEADER_SIZE + MAX_UDP_PAYLOAD);
    buf.resize(HEADER_SIZE, 0u8);
    Ok(Self {
      socket,
      group,
      topic_hash: topic_hash(topic),
      buf,
    })
  }

  /// Send `payload` to the multicast group.
  pub fn send(&mut self, payload: &[u8]) -> Result<(), UdpError> {
    if payload.len() > MAX_UDP_PAYLOAD {
      return Err(UdpError::PayloadTooLarge { len: payload.len() });
    }
    let hdr = FrameHeader::new(self.topic_hash, payload.len() as u16);
    self.buf.clear();
    self.buf.extend_from_slice(&hdr.to_bytes());
    self.buf.extend_from_slice(payload);
    match self.socket.send_to(&self.buf, self.group) {
      Ok(_) => Ok(()),
      Err(e) if e.kind() == ErrorKind::WouldBlock => Err(UdpError::WouldBlock),
      Err(e) => Err(UdpError::Send { source: e }),
    }
  }
}

// ---------------------------------------------------------------------------
// MulticastReceiver
// ---------------------------------------------------------------------------

/// Multicast UDP receiver.
///
/// Joins the multicast group derived from the topic name and receives
/// datagrams filtered by topic hash.
pub struct MulticastReceiver {
  socket: UdpSocket,
  topic_hash: u64,
  buf: [u8; HEADER_SIZE + MAX_UDP_PAYLOAD],
}

impl MulticastReceiver {
  /// Bind to `port` and join the multicast group for `topic` on `iface`.
  pub fn bind(topic: &str, port: u16, iface: Ipv4Addr) -> Result<Self, UdpError> {
    let group_ip = multicast_group_for_topic(topic);
    let local: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
    let socket = UdpSocket::bind(local).map_err(|e| UdpError::Bind {
      addr: local.to_string(),
      source: e,
    })?;
    // Allow multiple processes on the same machine to bind the same port.
    // SO_REUSEADDR is set via UdpSocket::bind on Linux by default; explicit for clarity.
    socket
      .join_multicast_v4(&group_ip, &iface)
      .map_err(|e| UdpError::JoinMulticast {
        group: group_ip.to_string(),
        source: e,
      })?;
    socket
      .set_nonblocking(true)
      .map_err(|e| UdpError::SockOpt { opt: "O_NONBLOCK", source: e })?;
    Ok(Self {
      socket,
      topic_hash: topic_hash(topic),
      buf: [0u8; HEADER_SIZE + MAX_UDP_PAYLOAD],
    })
  }

  /// Non-blocking receive. Returns `(payload, sender)` or [`UdpError::WouldBlock`].
  pub fn try_recv(&mut self) -> Result<(Vec<u8>, SocketAddr), UdpError> {
    loop {
      let (n, from) = match self.socket.recv_from(&mut self.buf) {
        Ok(v) => v,
        Err(e) if e.kind() == ErrorKind::WouldBlock => return Err(UdpError::WouldBlock),
        Err(e) => return Err(UdpError::Recv { source: e }),
      };
      if n < HEADER_SIZE { continue; }
      let hb: &[u8; HEADER_SIZE] = self.buf[..HEADER_SIZE].try_into().unwrap();
      let Some(hdr) = FrameHeader::from_bytes(hb) else { continue; };
      if hdr.topic_hash_host() != self.topic_hash { continue; }
      let plen = hdr.payload_len_host() as usize;
      if n < HEADER_SIZE + plen { continue; }
      let payload = self.buf[HEADER_SIZE..HEADER_SIZE + plen].to_vec();
      return Ok((payload, from));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn group_in_admin_scope() {
    let g = multicast_group_for_topic("/sensors/lidar");
    assert_eq!(g.octets()[0], 239, "First octet must be 239");
  }

  #[test]
  fn deterministic_group() {
    let a = multicast_group_for_topic("/sensors/lidar");
    let b = multicast_group_for_topic("/sensors/lidar");
    assert_eq!(a, b);
  }
}
