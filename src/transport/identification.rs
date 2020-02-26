use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Identification<T> {
    pub version: T,
    pub comment: T,
}

impl<T> Identification<T> {
    const PREFIX: &'static [u8] = b"SSH-2.0-";
    pub(crate) const MAX_LEN: usize = 253;

    pub fn new(version: T, comment: T) -> Self {
        Self { version, comment }
    }
}

impl Default for Identification<&'static str> {
    fn default() -> Self {
        Self {
            version: concat!(env!("CARGO_PKG_NAME"), "_", env!("CARGO_PKG_VERSION")),
            comment: "",
        }
    }
}

impl<T: AsRef<[u8]>> Encode for Identification<T> {
    fn size(&self) -> usize {
        Self::PREFIX.len()
            + self.version.as_ref().len()
            + if self.comment.as_ref().is_empty() {
                0
            } else {
                1 + self.comment.as_ref().len()
            }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_bytes(&Self::PREFIX);
        e.push_bytes(&self.version.as_ref());
        if !self.comment.as_ref().is_empty() {
            e.push_u8(' ' as u8);
            e.push_bytes(&self.comment.as_ref());
        }
    }
}

impl Decode for Identification<String> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_bytes(&Self::PREFIX)?;
        if d.remaining() > Self::MAX_LEN {
            return None;
        };
        Self {
            version: {
                let pred =
                    |x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != (' ' as u8);
                let version = d.take_while(pred)?;
                String::from_utf8(version.to_vec()).ok()?
            },
            comment: if d.is_eoi() {
                String::default()
            } else {
                d.expect_u8(' ' as u8)?;
                let comment = d.take_while(|x| (x as char).is_ascii_graphic())?;
                d.take_eoi()?;
                String::from_utf8(comment.to_vec()).ok()?
            },
        }
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_default_01() {
        let id = BEncoder::encode(&Identification::default());
        let id_ = concat!("SSH-2.0-", env!("CARGO_PKG_NAME"), "_", env!("CARGO_PKG_VERSION"));
        assert_eq!(id, id_.as_bytes());
    }

    #[test]
    fn test_encode_01() {
        let id: Identification<String> = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(b"SSH-2.0-ssh_0.1.0 ultra", &BEncoder::encode(&id)[..]);
    }

    /// Test the branch where the input is longer than MAX_LEN.
    #[test]
    fn test_decode_01() {
        let input = concat!("SSH-2.0-ssh_0.1.0 ultraaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(None, BDecoder::decode::<Identification<String>>(input.as_ref()));
    }

    #[test]
    fn test_decode_02() {
        let id = Identification::new("ssh_0.1.0".into(), "".into());
        assert_eq!(Some(id), BDecoder::decode(b"SSH-2.0-ssh_0.1.0"));
    }

    #[test]
    fn test_decode_03() {
        let id = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(Some(id), BDecoder::decode(b"SSH-2.0-ssh_0.1.0 ultra"));
    }
}
