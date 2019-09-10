use crate::codec::*;
use crate::keys::*;

#[derive(Clone, Debug)]
pub struct MsgIdentitiesAnswer {
    pub identities: Vec<(PublicKey,String)>
}

impl MsgIdentitiesAnswer {
    pub const MSG_NUMBER: u8 = 12;
}

impl <'a> Codec<'a> for MsgIdentitiesAnswer {
    fn size(&self) -> usize {
        1 + Codec::size(&self.identities)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.identities, e);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self {
            identities: Codec::decode(d)?
        }.into()
    }
}
