use crate::util::assume;
use crate::util::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Identification<T = String> {
    pub version: T,
    pub comment: T,
}

impl<T> Identification<T> {
    pub const PREFIX: &'static [u8] = b"SSH-2.0-";
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

impl From<Identification<&'static str>> for Identification {
    fn from(x: Identification<&'static str>) -> Self {
        Self {
            version: x.version.into(),
            comment: x.comment.into()
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
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_bytes(&Self::PREFIX)?;
        e.push_bytes(&self.version.as_ref())?;
        if !self.comment.as_ref().is_empty() {
            e.push_u8(' ' as u8)?;
            e.push_bytes(&self.comment.as_ref())?;
        }
        Some(())
    }
}

impl Decode for Identification<String> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_bytes(&Self::PREFIX)?;
        let pred = |x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != (' ' as u8);
        let version = d.take_while(pred)?;
        let d_ = d.clone();
        let comment = if d.expect_u8(b' ').is_some() {
            d.take_while(|x| (x as char).is_ascii_graphic())?
        } else {
            *d = d_;
            b""
        };
        assume(Self::PREFIX.len() + version.len() + comment.len() < Self::MAX_LEN)?;
        Self {
            version: String::from_utf8(version.to_vec()).ok()?,
            comment: String::from_utf8(comment.to_vec()).ok()?,
        }
        .into()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CrLf<T>(pub T);

impl <T: Encode> Encode for CrLf<&T> {
    fn size(&self) -> usize {
        self.0.size() + 2
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_encode(self.0)?;
        e.push_u8(b'\r')?;
        e.push_u8(b'\n')?;
        Some(())
    }
}

impl <T: Decode> Decode for CrLf<T> {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let t = Decode::decode(d)?;
        d.expect_u8(b'\r')?;
        d.expect_u8(b'\n')?;
        Some(CrLf(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_01() {
        let id = SliceEncoder::encode(&Identification::default());
        let id_ = concat!(
            "SSH-2.0-",
            env!("CARGO_PKG_NAME"),
            "_",
            env!("CARGO_PKG_VERSION")
        );
        assert_eq!(id, id_.as_bytes());
    }

    #[test]
    fn test_encode_01() {
        let id: Identification<String> = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(b"SSH-2.0-ssh_0.1.0 ultra", &SliceEncoder::encode(&id)[..]);
    }

    /// Test the branch where the input is longer than MAX_LEN.
    #[test]
    fn test_decode_01() {
        let input =
            concat!("SSH-2.0-ssh_0.1.0 ultraaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(
            None,
            SliceDecoder::decode::<Identification<String>>(input.as_ref())
        );
    }

    #[test]
    fn test_decode_02() {
        let id = Identification::new("ssh_0.1.0".into(), "".into());
        assert_eq!(Some(id), SliceDecoder::decode(b"SSH-2.0-ssh_0.1.0"));
    }

    #[test]
    fn test_decode_03() {
        let id = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(Some(id), SliceDecoder::decode(b"SSH-2.0-ssh_0.1.0 ultra"));
    }
}
