use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct ServiceRequest<'a> (pub &'a str);

impl <'a> ServiceRequest<'a> {
    const MSG_NUMBER: u8 = 5;

    pub fn user_auth() -> Self {
        Self("ssh-userauth")
    }

    pub fn connection() -> Self {
        Self("ssh-connection")
    }
}

impl<'a> Codec<'a> for ServiceRequest<'a> {
    fn size(&self) -> usize {
        1 + Codec::size(&self.0)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(Self::MSG_NUMBER as u8);
        Codec::encode(&self.0, c);
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &Self::MSG_NUMBER)?;
        Self (Codec::decode(d)?).into()
    }
}
