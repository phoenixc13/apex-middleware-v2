//! TCP listener: accepts incoming connections and yields [`TcpConnection`]s.

use std::net::{SocketAddr, TcpListener as StdTcpListener};

use crate::{
  connection::TcpConnection,
  error::TcpError,
};

/// Bound TCP server socket that accepts APEX peer connections.
///
/// Uses `std::net::TcpListener` in blocking mode. For high-connection-rate
/// scenarios, the caller should accept in a dedicated OS thread.
pub struct TcpListener {
  inner: StdTcpListener,
  local: SocketAddr,
}

impl TcpListener {
  /// Bind to `addr` and start listening.
  pub fn bind(addr: SocketAddr) -> Result<Self, TcpError> {
    let inner = StdTcpListener::bind(addr).map_err(|e| TcpError::Bind {
      addr: addr.to_string(),
      source: e,
    })?;
    let local = inner.local_addr().map_err(|e| TcpError::Os { source: e })?;
    Ok(Self { inner, local })
  }

  /// Local bound address.
  pub fn local_addr(&self) -> SocketAddr {
    self.local
  }

  /// Block until a new connection arrives, then return a configured
  /// [`TcpConnection`].
  pub fn accept(&self) -> Result<TcpConnection, TcpError> {
    let (stream, remote) = self.inner.accept().map_err(|e| TcpError::Accept { source: e })?;
    TcpConnection::from_stream(stream, remote)
  }

  /// Set the accept timeout (milliseconds). `0` disables the timeout.
  pub fn set_accept_timeout(&self, ms: u64) -> Result<(), TcpError> {
    use std::time::Duration;
    let t = if ms == 0 { None } else { Some(Duration::from_millis(ms)) };
    self.inner
      .set_nonblocking(false)
      .map_err(|e| TcpError::SockOpt { opt: "O_NONBLOCK", source: e })?;
    // TcpListener does not expose set_read_timeout directly; rely on accept()
    // OS-level timeout via SO_RCVTIMEO where available.
    #[cfg(unix)]
    if let Some(dur) = t {
      use std::os::unix::io::AsRawFd;
      let secs = dur.as_secs() as libc::time_t;
      let usecs = dur.subsec_micros() as libc::suseconds_t;
      let tv = libc::timeval { tv_sec: secs, tv_usec: usecs };
      unsafe {
        libc::setsockopt(
          self.inner.as_raw_fd(),
          libc::SOL_SOCKET,
          libc::SO_RCVTIMEO,
          &tv as *const _ as *const libc::c_void,
          std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        );
      }
    }
    Ok(())
  }
}
