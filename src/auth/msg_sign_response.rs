use crate::algorithm::auth::*;
use crate::codec::*;
use crate::message::*;

#[derive(Debug, PartialEq)]
pub struct MsgSignResponse {
    pub signature: Signature,
}

impl Message for MsgSignResponse {
    const NUMBER: u8 = 14;
}

impl Encode for MsgSignResponse {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.signature)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        Encode::encode(&self.signature, e);
    }
}

impl Decode for MsgSignResponse {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            signature: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgSignResponse {
            signature: Signature::Ed25519(SshEd25519Signature([3; 64])),
        };
        assert_eq!(
            vec![
                14, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0,
                0, 0, 64, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3
            ],
            BEncoder::encode(&msg)
        );
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgSignResponse {
            signature: Signature::Ed25519(SshEd25519Signature([3; 64])),
        };
        assert_eq!(
            Some(msg),
            BDecoder::decode(
                &[
                    14, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57,
                    0, 0, 0, 64, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3
                ][..]
            )
        );
    }
}