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
        1 + self.identities.size()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push(&self.identities)
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

    #[test]
    fn test_msg_identities_answer_encode_01() {
        let expected: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 0, 3, 1, 2, 3, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116,
            121, 32, 49, 0, 0, 0, 2, 4, 5, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116, 121, 32,
            50,
        ];
        let actual: Vec<u8> = SliceEncoder::encode(&MsgIdentitiesAnswer {
            identities: vec![
                (Identity::from(vec![1, 2, 3]), "identity 1".into()),
                (Identity::from(vec![4, 5]), "identity 2".into()),
            ],
        });
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_msg_identities_answer_decode_01() {
        let input: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 0, 3, 1, 2, 3, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116,
            121, 32, 49, 0, 0, 0, 2, 4, 5, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116, 121, 32,
            50,
        ];
        let actual: Option<MsgIdentitiesAnswer> = SliceDecoder::decode(input.as_ref());
        let expected = Some(MsgIdentitiesAnswer {
            identities: vec![
                (Identity::from(vec![1, 2, 3]), "identity 1".into()),
                (Identity::from(vec![4, 5]), "identity 2".into()),
            ],
        });
        assert_eq!(actual, expected);
    }
}
