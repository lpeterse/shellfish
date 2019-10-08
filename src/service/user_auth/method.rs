mod password;
mod publickey;

pub use password::*;
pub use publickey::*;

use crate::codec::*;

pub trait AuthMethod {
    const NAME: &'static str;
}
