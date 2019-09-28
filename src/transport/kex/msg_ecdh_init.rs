use super::*;
use crate::codec::*;

#[derive(Clone, Debug)]
pub struct KexEcdhInit<A: EcdhAlgorithm> {
    dh_public: A::PublicKey,
}

impl <A: EcdhAlgorithm> KexEcdhInit<A> {
    pub const MSG_NUMBER: u8 = 30;
}

impl<A: EcdhAlgorithm> KexEcdhInit<A> {
    pub fn new(dh_public: A::PublicKey) -> Self {
        Self { dh_public }
    }
}

impl<A: EcdhAlgorithm> Encode for KexEcdhInit<A>
where
    A::PublicKey: Encode
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.dh_public)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER);
        Encode::encode(&self.dh_public, c);
    }
}

impl<'a, A: EcdhAlgorithm> DecodeRef<'a> for KexEcdhInit<A>
where
    A::PublicKey: DecodeRef<'a>
{
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(Self::MSG_NUMBER)?;
        Self {
            dh_public: DecodeRef::decode(c)?,
        }.into()
    }
}
