mod session;
mod other;

pub use self::session::*;
pub use self::other::*;

use crate::codec::*;

pub trait ChannelType {
    type Request;
    type Confirmation;
    fn name() -> &'static str;
    fn size_request(x: &Self::Request) -> usize;
    fn encode_request<E: Encoder>(x: &Self::Request, e: &mut E);
    fn decode_request<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self::Request>;
    fn size_confirmation(x: &Self::Confirmation) -> usize;
    fn encode_confirmation<E: Encoder>(x: &Self::Confirmation, e: &mut E);
    fn decode_confirmation<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self::Confirmation>;
}
