use crate::codec::*;
use crate::keys::*;
use super::ecdh_algorithm::*;

#[derive(Clone, Debug)]
pub struct KexEcdhReply<A: EcdhAlgorithm> {
    pub host_key: PublicKey,
    pub dh_public: A::PublicKey,
    pub signature: Signature,
}

impl <A: EcdhAlgorithm> KexEcdhReply<A> {
    pub const MSG_NUMBER: u8 = 31;
}

impl <A: EcdhAlgorithm> Encode for KexEcdhReply<A>
where
    A::PublicKey: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
        + self.host_key.size()
        + self.dh_public.size()
        + self.signature.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER);
        Encode::encode(&self.host_key, e);
        Encode::encode(&self.dh_public, e);
        Encode::encode(&self.signature, e);
    }
}

impl <A: EcdhAlgorithm> Decode for KexEcdhReply<A>
where
    A::PublicKey: Decode,
{
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::MSG_NUMBER)?;
        Self {
            host_key: DecodeRef::decode(d)?,
            dh_public: DecodeRef::decode(d)?,
            signature: DecodeRef::decode(d)?,
        }.into()
    }
}
