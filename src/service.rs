pub mod connection;
pub mod user_auth;

use crate::role::Role;
use crate::transport::{Socket, Transport};

pub trait Service<R: Role> {
    const NAME: &'static str;

    fn new<S: Socket>(config: &R::Config, transport: Transport<R, S>) -> Self;
}
