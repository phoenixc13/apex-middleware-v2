//! Blocking TCP connection: send and receive APEX frames over a stream.

use std::{
  io::BufWriter,
  net::{SocketAddr, TcpStream},
  time::Duration,
};

use crate::{
  codec::{decode_frame, encode_frame},
  error::TcpError,
  DEFAULT_READ_TIMEOUT_MS, DEFAULT_WRITE_TIMEOUT_MS, TCP_KEEPALIVE_IDLE_SECS,
};

/// Blocking TCP connection wrapping a [`TcpStream`].
///
/// Each `TcpConnection` represents one established peer connection.
/// Send and receive operations are synchronous and subject to the
/// configured timeouts.
pub struct TcpConnection {
  stream: TcpStream,
  remote: SocketAddr,
}

impl TcpConnection {
  /// Establish a new connection to `remote`.
  pub fn connect(remote: SocketAddr) -> Result<Self, TcpError> {
    let stream = TcpStream::connect(remote).map_err(|e| TcpError::Connect {
      addr: remote.to_string(),
      source: e,
    })?;
    Self::configure_stream(&stream)?;
    Ok(Self { stream, remote })
  }

  /// Wrap an already-accepted stream.
  pub fn from_stream(stream: TcpStream, remote: SocketAddr) -> Result<Self, TcpError> {
    Self::configure_stream(&stream)?;
    Ok(Self { stream, remote })
  }

  /// Remote peer address.
  pub fn remote(&self) -> SocketAddr {
    self.remote
  }

  /// Send an APEX frame. `header_bytes` must be exactly 16 bytes.
  ///
  /// The write is fully buffered; the entire frame (prefix + header + payload)
  /// is flushed atomically.
  pub fn send(&mut self, header_bytes: &[u8; 16], payload: &[u8]) -> Result<(), TcpError> {
    let mut writer = BufWriter::new(&self.stream);
    encode_frame(&mut writer, header_bytes, payload)?;
    use std::io::Write;
    writer.flush().map_err(|e| TcpError::Write { source: e })
  }

  /// Receive one APEX frame. Blocks until a complete frame arrives or
  /// the read timeout expires.
  pub fn recv(&mut self) -> Result<([u8; 16], Vec<u8>), TcpError> {
    decode_frame(&mut self.stream)
  }

  /// Set custom read timeout (milliseconds). `0` means no timeout.
  pub fn set_read_timeout(&self, ms: u64) -> Result<(), TcpError> {
    let timeout = if ms == 0 { None } else { Some(Duration::from_millis(ms)) };
    self.stream
      .set_read_timeout(timeout)
      .map_err(|e| TcpError::SockOpt { opt: "SO_RCVTIMEO", source: e })
  }

  /// Set custom write timeout (milliseconds). `0` means no timeout.
  pub fn set_write_timeout(&self, ms: u64) -> Result<(), TcpError> {
    let timeout = if ms == 0 { None } else { Some(Duration::from_millis(ms)) };
    self.stream
      .set_write_timeout(timeout)
      .map_err(|e| TcpError::SockOpt { opt: "SO_SNDTIMEO", source: e })
  }

  // -------------------------------------------------------------------------
  // Internal helpers
  // -------------------------------------------------------------------------

  fn configure_stream(s: &TcpStream) -> Result<(), TcpError> {
    s.set_nodelay(true)
      .map_err(|e| TcpError::SockOpt { opt: "TCP_NODELAY", source: e })?;
    s.set_read_timeout(Some(Duration::from_millis(DEFAULT_READ_TIMEOUT_MS)))
      .map_err(|e| TcpError::SockOpt { opt: "SO_RCVTIMEO", source: e })?;
    s.set_write_timeout(Some(Duration::from_millis(DEFAULT_WRITE_TIMEOUT_MS)))
      .map_err(|e| TcpError::SockOpt { opt: "SO_SNDTIMEO", source: e })?;
    // TCP keepalive (best-effort; platform-specific).
    #[cfg(unix)]
    {
      use std::os::unix::io::AsRawFd;
      unsafe {
        let fd = s.as_raw_fd();
        let val: libc::c_int = 1;
        libc::setsockopt(
          fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE,
          &val as *const _ as *const libc::c_void,
          std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
        let idle = TCP_KEEPALIVE_IDLE_SECS as libc::c_int;
        libc::setsockopt(
          fd, libc::IPPROTO_TCP, libc::TCP_KEEPIDLE,
          &idle as *const _ as *const libc::c_void,
          std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
      }
    }
    Ok(())
  }
}
