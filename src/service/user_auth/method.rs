mod password;
mod pubkey;

pub use password::*;
pub use pubkey::*;

use crate::codec::*;

pub trait Method<'a>: Codec<'a> {
    const NAME: &'static str;
}
