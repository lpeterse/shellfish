use std::ffi::{OsStr, CStr};
use std::os::unix::prelude::{AsRawFd, OsStrExt, AsFd, FromRawFd};
use std::os::unix::prelude::OwnedFd;
use crate::util::check;

#[derive(Debug)]
pub struct Pty {
    ptm: OwnedFd,
    pts: OwnedFd,
}

impl Pty {
    pub fn new() -> Option<Self> {
        let mut buf: [libc::c_char; 512] = [0; 512];
        let ptm = unsafe {
            let ptm_fd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            check(ptm_fd != -1)?;
            // FIXME: unowned
            check(libc::grantpt(ptm_fd) != -1)?;
            check(libc::unlockpt(ptm_fd) != -1)?;
            std::fs::File::from_raw_fd(ptm_fd)
        };
        #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
        {
            if unsafe { libc::ptsname_r(ptm.as_raw_fd(), buf.as_mut_ptr(), buf.len()) } != 0 {
                return None
            }
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        unsafe {
            let st = libc::ptsname(ptm.as_raw_fd());
            if st.is_null() {
                return None
            }
            libc::strncpy(buf.as_mut_ptr(), st, buf.len());
        }

        let pts_name = OsStr::from_bytes(unsafe { CStr::from_ptr(&buf as _) }.to_bytes());
        let pts = std::fs::OpenOptions::new().read(true).write(true).open(pts_name).ok()?;
        Some(Self {
            ptm: ptm.as_fd().try_clone_to_owned().ok()?,
            pts: pts.as_fd().try_clone_to_owned().ok()?
        })
    }

    pub fn ptm(&self) -> &OwnedFd {
        &self.ptm
    }

    pub fn pts(&self) -> &OwnedFd {
        &self.pts
    }
}
