use crate::algorithm::*;
use crate::codec::*;
use crate::message::*;

#[derive(Debug, PartialEq)]
pub struct MsgSignResponse<S: AuthAlgorithm> {
    pub signature: S::AuthSignature,
}

impl<S: AuthAlgorithm> Message for MsgSignResponse<S> {
    const NUMBER: u8 = 14;
}

impl<S: AuthAlgorithm> Encode for MsgSignResponse<S>
where
    S::AuthSignature: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.signature)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        Encode::encode(&self.signature, e);
    }
}

impl<S: AuthAlgorithm> Decode for MsgSignResponse<S>
where
    S::AuthSignature: Decode,
{
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        log::error!("*");
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

    #[derive(Debug, PartialEq)]
    pub struct Foobar {}

    impl AuthAlgorithm for Foobar {
        const NAME: &'static str = "foobar";

        type AuthIdentity = ();
        type AuthSignature = String;
        type AuthSignatureFlags = u32;
    }

    #[test]
    fn test_encode_01() {
        let msg: MsgSignResponse<Foobar> = MsgSignResponse {
            signature: "SIGNATURE".into(),
        };
        assert_eq!(
            vec![14, 0, 0, 0, 9, 83, 73, 71, 78, 65, 84, 85, 82, 69],
            BEncoder::encode(&msg)
        );
    }

    #[test]
    fn test_decode_01() {
        let msg: MsgSignResponse<Foobar> = MsgSignResponse {
            signature: "SIGNATURE".into(),
        };
        assert_eq!(
            Some(msg),
            BDecoder::decode(&[14, 0, 0, 0, 9, 83, 73, 71, 78, 65, 84, 85, 82, 69][..])
        );
    }
}
