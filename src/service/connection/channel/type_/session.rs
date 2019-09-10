use super::*;
use crate::codec::*;

#[derive(Clone, Debug)]
pub struct Session {}

impl Session {
    pub const NAME: &'static str = "session";
}

impl ChannelType for Session {
    type Request = ();
    type Confirmation = ();

    fn name() -> &'static str {
        Self::NAME
    }
    fn size_request(_: &Self::Request) -> usize {
        0
    }
    fn encode_request<E: Encoder>(_: &Self::Request, _: &mut E) {}
    fn decode_request<'a, D: Decoder<'a>>(_: &mut D) -> Option<Self::Request> {
        Some(())
    }
    fn size_confirmation(_: &Self::Confirmation) -> usize {
        0
    }
    fn encode_confirmation<E: Encoder>(_: &Self::Confirmation, _: &mut E) {}
    fn decode_confirmation<'a, D: Decoder<'a>>(_: &mut D) -> Option<Self::Confirmation> {
        Some(())
    }
}
