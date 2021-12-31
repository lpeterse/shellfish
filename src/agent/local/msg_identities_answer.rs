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

impl SshEncode for MsgIdentitiesAnswer {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_usize(self.identities.len())?;
        for id in &self.identities {
            e.push(id)?;
        }
        Some(())
    }
}

impl SshDecode for MsgIdentitiesAnswer {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        let mut identities = vec![];
        d.expect_u8(<Self as Message>::NUMBER)?;
        let len = d.take_usize()?;
        for _ in 0..len {
            identities.push(d.take()?)
        }
        Some(Self { identities })
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
        let actual: Vec<u8> = SshCodec::encode(&MsgIdentitiesAnswer {
            identities: vec![
                (Identity::from(vec![1, 2, 3]), "identity 1".into()),
                (Identity::from(vec![4, 5]), "identity 2".into()),
            ],
        })
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_msg_identities_answer_decode_01() {
        let input: Vec<u8> = vec![
            12, 0, 0, 0, 2, 0, 0, 0, 3, 1, 2, 3, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116,
            121, 32, 49, 0, 0, 0, 2, 4, 5, 0, 0, 0, 10, 105, 100, 101, 110, 116, 105, 116, 121, 32,
            50,
        ];
        let actual = SshCodec::decode::<MsgIdentitiesAnswer>(input.as_ref());
        let expected = Ok(MsgIdentitiesAnswer {
            identities: vec![
                (Identity::from(vec![1, 2, 3]), "identity 1".into()),
                (Identity::from(vec![4, 5]), "identity 2".into()),
            ],
        });
        assert_eq!(actual, expected);
    }
}
