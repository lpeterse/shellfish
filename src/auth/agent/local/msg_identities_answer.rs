use super::*;
use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug, PartialEq)]
pub struct MsgIdentitiesAnswer {
    pub identities: Vec<(Identity, String)>,
}

impl Message for MsgIdentitiesAnswer {
    const NUMBER: u8 = 12;
}

impl Encode for MsgIdentitiesAnswer {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.identities)
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        Encode::encode(&self.identities, e)
    }
}

impl Decode for MsgIdentitiesAnswer {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            identities: Decode::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ssh_rsa::*;
    use super::super::ssh_ed25519::*;

    #[test]
    fn test_msg_identities_answer_encode_01() {
        let expected: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 0, 27, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3,
            1, 2, 3, 0, 0, 0, 5, 4, 5, 6, 7, 8, 0, 0, 0, 42, 47, 117, 115, 114, 47, 108, 105, 98,
            47, 120, 56, 54, 95, 54, 52, 45, 108, 105, 110, 117, 120, 45, 103, 110, 117, 47, 111,
            112, 101, 110, 115, 99, 45, 112, 107, 99, 115, 49, 49, 46, 115, 111, 0, 0, 0, 51, 0, 0,
            0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 111, 31, 72, 196,
            30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178, 182, 197, 240, 88, 108, 167,
            36, 126, 242, 16, 190, 192, 165, 40, 63, 0, 0, 0, 12, 114, 115, 115, 104, 45, 101, 120,
            97, 109, 112, 108, 101,
        ];
        let actual: Vec<u8> = SliceEncoder::encode(&MsgIdentitiesAnswer {
            identities: vec![
                (
                    Identity::RsaPublicKey(RsaPublicKey {
                        public_e: vec![1, 2, 3],
                        public_n: vec![4, 5, 6, 7, 8],
                    }),
                    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so".into(),
                ),
                (
                    Identity::Ed25519PublicKey(Ed25519PublicKey([
                        111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178,
                        182, 197, 240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63,
                    ])),
                    "rssh-example".into(),
                ),
            ],
        });
        assert_eq!(actual, expected);
    }

    // FIXME: uncomment test
    /*
    #[test]
    fn test_msg_identities_answer_decode_01() {
        let input: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 1, 23, 0, 0, 0, 7, 115, 115, 104, 45, 114, 115, 97, 0, 0, 0, 3,
            1, 2, 3, 0, 0, 0, 5, 4, 5, 6, 7, 8, 0, 0, 0, 42, 47, 117, 115, 114, 47, 108, 105, 98,
            47, 120, 56, 54, 95, 54, 52, 45, 108, 105, 110, 117, 120, 45, 103, 110, 117, 47, 111,
            112, 101, 110, 115, 99, 45, 112, 107, 99, 115, 49, 49, 46, 115, 111, 0, 0, 0, 51, 0, 0,
            0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0, 0, 0, 32, 111, 31, 72, 196,
            30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178, 182, 197, 240, 88, 108, 167,
            36, 126, 242, 16, 190, 192, 165, 40, 63, 0, 0, 0, 12, 114, 115, 115, 104, 45, 101, 120,
            97, 109, 112, 108, 101,
        ];
        let actual: Option<MsgIdentitiesAnswer> = SliceDecoder::decode(input.as_ref());
        let expected = Some(MsgIdentitiesAnswer {
            identities: vec![
                (
                    Identity::RsaPublicKey(RsaPublicKey {
                        public_e: vec![1, 2, 3],
                        public_n: vec![4, 5, 6, 7, 8],
                    }),
                    "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so".into(),
                ),
                (
                    Identity::Ed25519PublicKey(Ed25519PublicKey([
                        111, 31, 72, 196, 30, 64, 80, 99, 68, 115, 76, 34, 71, 49, 174, 174, 178,
                        182, 197, 240, 88, 108, 167, 36, 126, 242, 16, 190, 192, 165, 40, 63,
                    ])),
                    "rssh-example".into(),
                ),
            ],
        });
        assert_eq!(actual, expected);
    }
    */
}
