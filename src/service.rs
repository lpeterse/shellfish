pub mod user_auth;
pub mod connection;

use crate::role::Role;
use crate::transport::Transport;

use async_std::net::TcpStream;

pub trait Service<R: Role> {
    const NAME: &'static str;

    fn new(transport: Transport<R, TcpStream>) -> Self;
}
