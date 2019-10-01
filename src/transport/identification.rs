use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Identification {
    pub version: String,
    pub comment: String,
}

impl Identification {
    pub const MAX_LEN: usize = 253;
    const PREFIX: &'static [u8] = b"SSH-2.0-";

    pub fn new(version: String, comment: String) -> Self {
        Self { version, comment }
    }
}

impl Default for Identification {
    fn default() -> Self {
        Self {
            version: format!("{}_{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            comment: String::default(),
        }
    }
}

impl Encode for Identification {
    fn size(&self) -> usize {
        Self::PREFIX.len()
            + self.version.len()
            + if self.comment.is_empty() {
                0
            } else {
                1 + self.comment.len()
            }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_bytes(&Self::PREFIX);
        e.push_bytes(&self.version.as_bytes());
        if !self.comment.is_empty() {
            e.push_u8(' ' as u8);
            e.push_bytes(&self.comment.as_bytes());
        }
    }
}

impl Decode for Identification {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_match(&Self::PREFIX)?;
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
    fn test_encode_01() {
        let id = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(b"SSH-2.0-ssh_0.1.0 ultra", &BEncoder::encode(&id)[..]);
    }

    #[test]
    fn test_decode_01() {
        let id = Identification::new("ssh_0.1.0".into(), "ultra".into());
        assert_eq!(Some(id), BDecoder::decode(b"SSH-2.0-ssh_0.1.0 ultra"));
    }
}
