use crate::codec::*;

use crate::algorithm::*;

#[derive(Debug, PartialEq)]
pub struct MsgSignResponse<S: SignatureAlgorithm> {
    pub signature: S::Signature,
}

impl <S: SignatureAlgorithm> MsgSignResponse<S> {
    pub const MSG_NUMBER: u8 = 14;
}

impl <S: SignatureAlgorithm> Encode for MsgSignResponse<S>
where
    S::Signature: Encode
{
    fn size(&self) -> usize {
        1 + Encode::size(&self.signature)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(&self.signature, e);
    }
}

impl <S: SignatureAlgorithm> Decode for MsgSignResponse<S>
where
    S::Signature: Decode
{
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::MSG_NUMBER)?;
        Self {
            signature: DecodeRef::decode(d)?
        }.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub struct Foobar {}

    impl SignatureAlgorithm for Foobar {
        const NAME: &'static str = "foobar";

        type PublicKey = ();
        type PrivateKey = ();
        type Signature = String;
        type SignatureFlags = u32;
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
        assert_eq!(Some(msg),
            BDecoder::decode(&[14, 0, 0, 0, 9, 83, 73, 71, 78, 65, 84, 85, 82, 69][..])
        );
    }
}
