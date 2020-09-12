use async_std::net::TcpStream;

#[derive(Clone, Debug)]
pub struct TcpKeepaliveConfig {
    pub time: Option<std::time::Duration>,
    pub intvl: Option<std::time::Duration>,
    pub probes: Option<usize>,
}

impl TcpKeepaliveConfig {
    /// TODO: Remove as soon as this is supported by std.
    pub fn apply(&self, socket: &TcpStream) -> Result<(), std::io::Error> {
        use libc::{c_int, c_void, socklen_t};
        use libc::{SOL_SOCKET, SOL_TCP, SO_KEEPALIVE};
        use libc::{TCP_KEEPCNT, TCP_KEEPIDLE, TCP_KEEPINTVL};
        use std::os::unix::io::AsRawFd;
        fn set_opt(sock: c_int, opt: c_int, val: c_int, payload: c_int) -> c_int {
            unsafe {
                libc::setsockopt(
                    sock,
                    opt,
                    val,
                    &payload as *const c_int as *const c_void,
                    std::mem::size_of::<c_int>() as socklen_t,
                )
            }
        }
        let fd = socket.as_raw_fd();
        let msg = "Setting TCP keepalive failed";
        let err = Err(std::io::Error::new(std::io::ErrorKind::Other, msg));

        if set_opt(fd, SOL_SOCKET, SO_KEEPALIVE, 1) != 0 {
            return err;
        }
        if let Some(x) = self.time {
            if set_opt(fd, SOL_TCP, TCP_KEEPIDLE, x.as_secs() as c_int) != 0 {
                return err;
            }
        }
        if let Some(x) = self.intvl {
            if set_opt(fd, SOL_TCP, TCP_KEEPINTVL, x.as_secs() as c_int) != 0 {
                return err;
            }
        }
        if let Some(x) = self.probes {
            if set_opt(fd, SOL_TCP, TCP_KEEPCNT, x as c_int) != 0 {
                return err;
            }
        }
        Ok(())
    }
}

impl Default for TcpKeepaliveConfig {
    fn default() -> Self {
        Self {
            time: Some(std::time::Duration::from_secs(300)),
            intvl: Some(std::time::Duration::from_secs(5)),
            probes: Some(5),
        }
    }
}
