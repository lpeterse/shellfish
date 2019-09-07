use crate::codec::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct KexEcdhReply {
    pub host_key: PublicKey,
    pub dh_public: Vec<u8>,
    pub signature: Signature,
}

impl <'a> Codec<'a> for KexEcdhReply {
    fn size(&self) -> usize {
        1
        + self.host_key.size()
        + self.dh_public.size()
        + self.signature.size()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(31);
        Codec::encode(&self.host_key, c);
        Codec::encode(&self.dh_public, c);
        Codec::encode(&self.signature, c);
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u8().filter(|x| x == &31)?;
        let hk = Codec::decode(c)?;
        let ek = Codec::decode(c)?;
        let sig = Codec::decode(c)?;
        Some(Self {
            host_key: hk,
            dh_public: ek,
            signature: sig,
        })
    }
}
