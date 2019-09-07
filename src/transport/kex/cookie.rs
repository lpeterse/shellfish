use rand_core::RngCore;
use rand_os::OsRng;

#[derive(Clone,Copy)]
pub struct KexCookie (pub [u8;16]);

impl KexCookie {
    pub fn random() -> Self {
        let mut cookie: [u8;16] = [0;16];
        OsRng::new().unwrap().fill_bytes(&mut cookie);
        Self(cookie)
    }
}

impl AsRef<[u8]> for KexCookie {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for KexCookie {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KexCookie ({:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x})",
            self.0[00], self.0[01], self.0[02], self.0[03],
            self.0[04], self.0[05], self.0[06], self.0[07],
            self.0[08], self.0[09], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15])
    }
}
