use crate::codec::*;

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

impl <'a> Codec<'a> for KexEcdhInit {
    fn size(&self) -> usize {
        1 + Codec::size(&self.dh_public)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(30);
        Codec::encode(&self.dh_public, c);
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u8().filter(|x| x == &30)?;
        Some(Self {
            dh_public: Codec::decode(c)?
        })
    }
}
