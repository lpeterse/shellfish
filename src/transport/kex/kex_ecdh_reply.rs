use crate::codec::*;
use crate::codec_ssh::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct KexEcdhReply {
    pub host_key: PublicKey,
    pub dh_public: Vec<u8>,
    pub signature: Signature,
}

impl <'a> SshCodec<'a> for KexEcdhReply {
    fn size(&self) -> usize {
        1
        + self.host_key.size()
        + self.dh_public.size()
        + self.signature.size()
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u8(31);
        SshCodec::encode(&self.host_key, c);
        SshCodec::encode(&self.dh_public, c);
        SshCodec::encode(&self.signature, c);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        c.take_u8().filter(|x| x == &31)?;
        let hk = SshCodec::decode(c)?;
        let ek = SshCodec::decode(c)?;
        let sig = SshCodec::decode(c)?;
        Some(Self {
            host_key: hk,
            dh_public: ek,
            signature: sig,
        })
    }
}
