mod password;
mod publickey;

pub use password::*;
pub use publickey::*;

use crate::util::codec::*;

pub trait AuthMethod {
    const NAME: &'static str;
}
