use super::ecdh_algorithm::*;
use super::*;
use crate::algorithm::authentication::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgKexEcdhReply<A: EcdhAlgorithm, HI> {
    pub host_key: HI,
    pub dh_public: A::PublicKey,
    pub signature: HostSignature,
}

impl<A: EcdhAlgorithm, HI> Message for MsgKexEcdhReply<A, HI> {
    const NUMBER: u8 = 31;
}

impl<A, HI> Encode for MsgKexEcdhReply<A, HI>
where
    A: EcdhAlgorithm,
    A::PublicKey: Encode,
    HI: Encode,
{
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
            + self.host_key.size()
            + self.dh_public.size()
            + self.signature.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.host_key, e);
        Encode::encode(&self.dh_public, e);
        Encode::encode(&self.signature, e);
    }
}

impl<A: EcdhAlgorithm, HI> Decode for MsgKexEcdhReply<A, HI>
where
    A: EcdhAlgorithm,
    A::PublicKey: Decode,
    HI: Decode,
{
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            host_key: DecodeRef::decode(d)?,
            dh_public: DecodeRef::decode(d)?,
            signature: DecodeRef::decode(d)?,
        }
        .into()
    }
}
