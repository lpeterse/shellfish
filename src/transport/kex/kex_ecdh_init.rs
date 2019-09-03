use crate::codec::*;
use crate::codec_ssh::*;

#[derive(Clone, Debug)]
pub struct KexEcdhInit {
    dh_public: Vec<u8>
}

impl KexEcdhInit {
    pub fn new(dh_public: &[u8]) -> Self {
        Self {
            dh_public: Vec::from(dh_public)
        }
    }
}

impl <'a> SshCodec<'a> for KexEcdhInit {
    fn size(&self) -> usize {
        1 + SshCodec::size(&self.dh_public)
    }
    fn encode(&self, c: &mut Encoder<'a>) {
        c.push_u8(30);
        SshCodec::encode(&self.dh_public, c);
    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        c.take_u8().filter(|x| x == &30)?;
        Some(Self {
            dh_public: SshCodec::decode(c)?
        })
    }
}
