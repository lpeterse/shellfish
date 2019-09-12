mod password;
mod publickey;

pub use password::*;
pub use publickey::*;

use crate::codec::*;

pub trait Method<'a>: Codec<'a> {
    const NAME: &'static str;
}
