mod none;
mod password;
mod publickey;

pub use none::*;
pub use password::*;
pub use publickey::*;

use crate::util::codec::*;

pub trait AuthMethod {
    const NAME: &'static str;
}
