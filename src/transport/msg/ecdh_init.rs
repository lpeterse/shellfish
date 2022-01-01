use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgEcdhInit {
    pub dh_public: Vec<u8>,
}

impl MsgEcdhInit {
    pub fn new(dh_public: Vec<u8>) -> Self {
        Self { dh_public }
    }
}

impl Message for MsgEcdhInit {
    const NUMBER: u8 = 30;
}

impl SshEncode for MsgEcdhInit {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_bytes_framed(&self.dh_public)
    }
}

impl SshDecode for MsgEcdhInit {
    fn decode<'a, D: SshDecoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            dh_public: c.take_bytes_framed()?.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgEcdhInit {
            dh_public: vec![1, 2, 3],
        };
        let bytes = [30, 0, 0, 0, 3, 1, 2, 3];
        assert_eq!(&bytes, &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgEcdhInit {
            dh_public: vec![1, 2, 3],
        };
        let bytes = [30, 0, 0, 0, 3, 1, 2, 3];
        assert_eq!(&Ok(msg), &SshCodec::decode(&bytes));
    }
}
