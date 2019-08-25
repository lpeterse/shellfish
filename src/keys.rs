use crate::codec::*;
use crate::codec_ssh::*;

#[derive(Clone, Debug)]
pub enum PublicKey {
    RsaPublicKey(RsaPublicKey),
    UnknownPublicKey(UnknownPublicKey),
}

#[derive(Clone, Debug)]
pub struct RsaPublicKey (Vec<u8>);

#[derive(Clone, Debug)]
pub struct UnknownPublicKey {
    algo: String,
    key: Vec<u8>,
}

impl <'a> SshCodec<'a> for PublicKey {
    fn size(&self) -> usize {
        panic!("")
    }
    fn encode(&self,c: &mut Encoder<'a>) {
        panic!("")
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        let mut dec = c.take_decoder(len as usize)?;
        Some(match SshCodec::decode(&mut dec)? {
            "ssh-rsa" => PublicKey::RsaPublicKey(RsaPublicKey(Vec::from(dec.take_all()?))),
            algo      => PublicKey::UnknownPublicKey(UnknownPublicKey {
                algo: String::from(algo),
                key:  Vec::from(dec.take_all()?),
            }),
        })
    }
}

