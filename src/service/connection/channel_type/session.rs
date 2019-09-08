use super::*;
use crate::codec::*;

#[derive(Clone,Debug)]
pub struct Session {}

impl Session {
    pub const NAME: &'static str = "session";
}

impl <'a> ChannelType<'a> for Session {
    type Open = SessionData;
    type Confirmation = SessionData;
}

impl <'a> Codec<'a> for SessionData {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        // Nothing to do
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self {}.into()
    }
}

#[derive(Clone,Debug)]
pub struct SessionData {}

impl <'a> Named<'a> for SessionData {
    fn name(&self) -> &'a str {
        Session::NAME
    }
    fn decode<D: Decoder<'a>>(d: &mut D, name: &str) -> Option<Self> {
        Self {}.into()
    }
}
