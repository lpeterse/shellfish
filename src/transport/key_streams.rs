use sha2::{Digest, Sha256};

#[derive(Debug)]
pub enum KeyStreams {
    KeyStreamsSha256(KeyStreamsSha256),
}

impl KeyStreams {
    pub fn new_sha256<S: AsRef<[u8]>>(k: &[u8], h: &[u8], sid: S) -> Self {
        Self::KeyStreamsSha256(KeyStreamsSha256::new(k, h, sid.as_ref()))
    }

    pub fn c<'a>(&'a mut self) -> KeyStream<'a> {
        match self {
            Self::KeyStreamsSha256(ks) => KeyStream::KeyStreamSha256(ks.c()),
        }
    }

    pub fn d<'a>(&'a mut self) -> KeyStream<'a> {
        match self {
            Self::KeyStreamsSha256(ks) => KeyStream::KeyStreamSha256(ks.d()),
        }
    }
}

pub struct KeyStreamsSha256 {
    h: Vec<u8>,
    k: Vec<u8>,
    sid: Vec<u8>,
    state: Sha256,
}

impl KeyStreamsSha256 {
    fn new(k: &[u8], h: &[u8], sid: &[u8]) -> Self {
        Self {
            k: Vec::from(k),
            h: Vec::from(h),
            sid: Vec::from(sid),
            state: Sha256::new(),
        }
    }

    fn c<'a>(&'a mut self) -> KeyStreamSha256<'a> {
        KeyStreamSha256::new(&mut self.state, &self.k, &self.h, &self.sid, 'C')
    }

    fn d<'a>(&'a mut self) -> KeyStreamSha256<'a> {
        KeyStreamSha256::new(&mut self.state, &self.k, &self.h, &self.sid, 'D')
    }
}

impl std::fmt::Debug for KeyStreamsSha256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KexKeyStreams256 (..)")
    }
}

pub enum KeyStream<'a> {
    KeyStreamSha256(KeyStreamSha256<'a>),
}

impl<'a> KeyStream<'a> {
    pub fn read(&mut self, buf: &mut [u8]) {
        match self {
            KeyStream::KeyStreamSha256(ks) => ks.read(buf),
        }
    }
}

pub struct KeyStreamSha256<'a> {
    k: &'a [u8],
    h: &'a [u8],
    state: &'a mut Sha256,
    stream: Vec<u8>,
    position: usize,
}

impl<'a> KeyStreamSha256<'a> {
    const DIGEST_SIZE: usize = 32;

    fn new(state: &'a mut Sha256, k: &'a [u8], h: &'a [u8], sid: &'a [u8], idx: char) -> Self {
        // RFC: "Here K is encoded as mpint and "A" as byte and session_id as raw
        //       data.  "A" means the single character A, ASCII 65."
        Self::input_as_mpint(state, k);
        state.input(h);
        state.input([idx as u8]);
        state.input(sid);
        let mut stream = Vec::with_capacity(2 * Self::DIGEST_SIZE);
        stream.extend_from_slice(&state.result_reset());
        Self {
            k,
            h,
            state,
            stream,
            position: 0,
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) {
        let requested = buf.len();
        let mut available = self.stream.len() - self.position;
        while requested > available {
            Self::input_as_mpint(&mut self.state, self.k);
            self.state.input(self.h);
            self.state.input(&self.stream);
            self.stream
                .extend_from_slice(&self.state.result_reset()[..]);
            available += Self::DIGEST_SIZE;
        }
        buf.copy_from_slice(&self.stream[self.position..self.position + requested]);
        self.position += requested;
    }

    fn input_as_mpint(s: &mut Sha256, k: &[u8]) {
        if !k.is_empty() && k[0] > 127 {
            let l = k.len() + 1;
            s.input([(l >> 24) as u8, (l >> 16) as u8, (l >> 8) as u8, l as u8, 0]);
        } else {
            let l = k.len();
            s.input([(l >> 24) as u8, (l >> 16) as u8, (l >> 8) as u8, l as u8]);
        }
        s.input(k);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_key_streams_sha2_01() {
        let k = [
            107, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ];
        let h = [
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ];
        let sid = h;
        let c1 = [
            35, 83, 168, 202, 23, 231, 195, 6, 115, 123, 255, 191, 43, 255, 229, 67, 98, 137, 190,
            144, 108, 174, 108, 161, 250, 15, 170, 67, 142, 10, 102, 230,
        ];
        let c2 = [
            208, 1, 211, 152, 131, 216, 216, 233, 134, 111, 193, 40, 199, 147, 160, 146, 106, 253,
            28, 52, 89, 128, 0, 225, 61, 213, 79, 108, 116, 63, 4, 20,
        ];
        let c3 = [
            94, 43, 166, 97, 133, 172, 152, 2, 25, 11, 160, 232, 103, 139, 27, 114, 221, 159, 190,
            65, 59, 46, 138, 76, 237, 254, 51, 56, 37, 43, 86, 130,
        ];
        let d1 = [
            184, 79, 161, 100, 101, 46, 182, 213, 100, 94, 243, 107, 150, 176, 38, 24, 244, 253,
            153, 109, 83, 174, 5, 231, 139, 201, 30, 78, 88, 167, 227, 41,
        ];
        let d2 = [
            172, 158, 149, 34, 60, 215, 124, 232, 242, 63, 133, 44, 219, 188, 109, 5, 24, 230, 203,
            243, 189, 84, 85, 5, 162, 99, 163, 190, 201, 197, 78, 27,
        ];
        let d3 = [
            22, 59, 101, 191, 103, 154, 108, 11, 161, 26, 111, 59, 28, 53, 216, 56, 218, 138, 28,
            112, 145, 205, 14, 56, 169, 159, 134, 220, 119, 201, 5, 244,
        ];

        let mut ks = KeyStreams::new_sha256(&k[..], &h[..], &sid[..]);

        let mut c = ks.c();
        let mut c1_ = [0; 32];
        c.read(&mut c1_);
        assert_eq!(c1, c1_, "c1");
        let mut c2_ = [0; 32];
        c.read(&mut c2_);
        assert_eq!(c2, c2_, "c2");
        let mut c3_ = [0; 32];
        c.read(&mut c3_);
        assert_eq!(c3, c3_, "c3");

        let mut d = ks.d();
        let mut d1_ = [0; 32];
        d.read(&mut d1_);
        assert_eq!(d1, d1_, "d1");
        let mut d2_ = [0; 32];
        d.read(&mut d2_);
        assert_eq!(d2, d2_, "d2");
        let mut d3_ = [0; 32];
        d.read(&mut d3_);
        assert_eq!(d3, d3_, "d3");
    }

    #[test]
    fn test_key_streams_sha2_02() {
        let k = [
            207, 228, 126, 33, 91, 152, 255, 218, 241, 220, 23, 167, 79, 146, 12, 100, 222, 142,
            141, 72, 246, 81, 24, 199, 127, 89, 24, 29, 124, 166, 187, 14,
        ];
        //  ^ first byte of k is > 127
        let h = [
            143, 162, 77, 88, 20, 122, 164, 90, 216, 15, 8, 149, 23, 47, 66, 157, 242, 12, 176, 63,
            153, 120, 103, 133, 17, 36, 10, 69, 6, 145, 250, 211,
        ];
        let sid = h;
        let c1 = [
            125, 246, 53, 208, 237, 52, 170, 30, 97, 138, 151, 151, 199, 53, 83, 108, 130, 235,
            231, 87, 227, 10, 212, 137, 52, 16, 100, 244, 188, 104, 75, 76,
        ];

        let mut ks = KeyStreams::new_sha256(&k[..], &h[..], &sid[..]);

        let mut c = ks.c();
        let mut c1_ = [0; 32];
        c.read(&mut c1_);
        assert_eq!(c1, c1_, "c1");
    }

    #[test]
    fn test_key_streams_debug_01() {
        let k = [];
        let h = [];
        let sid = h;

        let ks = KeyStreams::new_sha256(&k[..], &h[..], &sid[..]);
        assert_eq!(
            "KeyStreamsSha256(KexKeyStreams256 (..))",
            format!("{:?}", ks)
        )
    }
}
