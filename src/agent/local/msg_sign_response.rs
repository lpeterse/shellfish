use super::*;
use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug, PartialEq)]
pub struct MsgSignResponse {
    pub signature: Signature,
}

impl Message for MsgSignResponse {
    const NUMBER: u8 = 14;
}

impl SshEncode for MsgSignResponse {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push(&self.signature)
    }
}

impl SshDecode for MsgSignResponse {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let signature = d.take()?;
        Some(Self { signature })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgSignResponse {
            signature: Signature::new("ssh-ed25519".into(), vec![3; 64]),
        };
        assert_eq!(
            vec![
                14, 0, 0, 0, 83, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0,
                0, 0, 64, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
                3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3
            ],
            SshCodec::encode(&msg).unwrap()
        );
    }

    #[test]
    fn test_decode_01() {
        let msg = MsgSignResponse {
            signature: Signature::new("ssh-ed25519".into(), vec![3; 64]),
        };
        assert_eq!(
            Ok(msg),
            SshCodec::decode(
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
