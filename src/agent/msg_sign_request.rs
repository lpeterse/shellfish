use crate::algorithm::*;
use crate::codec::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a, S: SignatureAlgorithm, D: Encode> {
    pub key: &'a S::PublicKey,
    pub data: &'a D,
    pub flags: S::SignatureFlags,
}

impl<'a, S: SignatureAlgorithm, D: Encode> MsgSignRequest<'a, S, D> {
    pub const MSG_NUMBER: u8 = 13;
}

impl<'a, S: SignatureAlgorithm, D: Encode> Encode for MsgSignRequest<'a, S, D>
where
    S::PublicKey: Encode,
    S::Signature: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
            + Encode::size(self.key)
            + std::mem::size_of::<u32>()
            + Encode::size(self.data)
            + std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Encode::encode(self.key, e);
        e.push_u32be(Encode::size(self.data) as u32);
        Encode::encode(self.data, e);
        e.push_u32be(self.flags.into());
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct Foobar {}

    impl SignatureAlgorithm for Foobar {
        const NAME: &'static str = "foobar";

        type PublicKey = ();
        type PrivateKey = ();
        type Signature = ();
        type SignatureFlags = u32;
    }

    #[test]
    fn test_encode_01() {
        let data: &'static str = "data";
        let key = ();
        let msg: MsgSignRequest<Foobar, _> = MsgSignRequest {
            key: &key,
            data: &data,
            flags: 123,
        };
        assert_eq!(
            vec![13, 0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97, 0, 0, 0, 123],
            BEncoder::encode(&msg)
        );
    }
}
