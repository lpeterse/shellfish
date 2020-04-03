pub mod connection;
pub mod user_auth;

use crate::role::Role;
use crate::transport::TransportLayer;

pub trait Service<R: Role> {
    const NAME: &'static str;

    fn new<T: TransportLayer>(config: &R::Config, transport: T) -> Self;
}
