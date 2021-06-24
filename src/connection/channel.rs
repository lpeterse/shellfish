pub(crate) mod direct_tcpip;
pub(crate) mod open_failure;
pub(crate) mod session;
pub(crate) mod state;

pub use self::direct_tcpip::{DirectTcpIp, DirectTcpIpParams, DirectTcpIpRequest};
pub use self::open_failure::OpenFailure;
pub use self::session::Session;

pub trait Channel: Unpin + Sized {
    const NAME: &'static str;
}
