use super::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgKexEcdhInit<A: EcdhAlgorithm> {
    dh_public: A::PublicKey,
}

impl<A: EcdhAlgorithm> MsgKexEcdhInit<A> {
    pub fn new(dh_public: A::PublicKey) -> Self {
        Self { dh_public }
    }
}

impl <A: EcdhAlgorithm> Message for MsgKexEcdhInit<A> {
    const NUMBER: u8 = 30;
}

impl<A: EcdhAlgorithm> Encode for MsgKexEcdhInit<A>
where
    A::PublicKey: Encode
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>() + Encode::size(&self.dh_public)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.dh_public, c);
    }
}

impl<A: EcdhAlgorithm> Decode for MsgKexEcdhInit<A>
where
    A::PublicKey: Decode
{
    fn decode<'a, D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            dh_public: DecodeRef::decode(c)?,
        }.into()
    }
}
