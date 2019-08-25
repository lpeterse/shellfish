use crate::codec::*;
use crate::codec_ssh::*;

pub use self::rsa::*;
pub use self::unknown::*;

mod rsa;
mod unknown;

#[derive(Clone, Debug)]
pub enum PublicKey {
    RsaPublicKey(RsaPublicKey),
    UnknownPublicKey(UnknownPublicKey),
}

impl <'a> SshCodec<'a> for PublicKey {
    fn size(&self) -> usize {
        4 + match self {
            PublicKey::RsaPublicKey(k) => SshCodec::size(&"ssh-rsa") + k.size(),
            PublicKey::UnknownPublicKey(k) => SshCodec::size(&k.algo) + k.key.len(),
        }
    }
    fn encode(&self,c: &mut Encoder<'a>) {
        c.push_u32be((self.size() - 4) as u32);
        match self {
            PublicKey::RsaPublicKey(k) => {
                SshCodec::encode(&"ssh-rsa", c);
                SshCodec::encode(k, c);
            },
            PublicKey::UnknownPublicKey(k) => {
                SshCodec::encode(&k.algo, c);
                c.push_bytes(k.key.as_slice());
            }
        }
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let len = c.take_u32be()?;
        let mut dec = c.take_decoder(len as usize)?;
        Some(match SshCodec::decode(&mut dec)? {
            "ssh-rsa" => PublicKey::RsaPublicKey(SshCodec::decode(&mut dec)?),
            algo      => PublicKey::UnknownPublicKey(UnknownPublicKey {
                algo: String::from(algo),
                key:  Vec::from(dec.take_all()?),
            }),
        })
    }
}
