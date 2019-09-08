mod session;
mod other;

pub use self::session::*;
pub use self::other::*;

use crate::codec::*;

pub trait Named<'a>: Sized {
    fn name(&self) -> &'a str;
    fn decode<D: Decoder<'a>>(d: &mut D, name: &str) -> Option<Self>;
}

pub trait ChannelType<'a> {
    type Open: Codec<'a> + Named<'a>;
    type Confirmation: Codec<'a> + Named<'a>;
}
