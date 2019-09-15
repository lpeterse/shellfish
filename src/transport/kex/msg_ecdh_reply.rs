use crate::codec::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct KexEcdhReply {
    pub host_key: PublicKey,
    pub dh_public: Vec<u8>,
    pub signature: Signature,
}

impl Encode for KexEcdhReply {
    fn size(&self) -> usize {
        1
        + self.host_key.size()
        + self.dh_public.size()
        + self.signature.size()
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(31);
        Encode::encode(&self.host_key, c);
        Encode::encode(&self.dh_public, c);
        Encode::encode(&self.signature, c);
    }
}

impl <'a> Decode<'a> for KexEcdhReply {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u8().filter(|x| x == &31)?;
        let hk = Decode::decode(c)?;
        let ek = Decode::decode(c)?;
        let sig = Decode::decode(c)?;
        Some(Self {
            host_key: hk,
            dh_public: ek,
            signature: sig,
        })
    }
}
