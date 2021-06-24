mod client;
mod exit;
mod process;
mod server;
mod state_client;
mod state_server;
mod pty;

pub use self::process::*;
pub use self::client::*;
pub use self::server::*;
pub use self::state_client::*;
pub use self::state_server::*;
pub use self::pty::*;

use super::Channel;
use super::OpenFailure;
use crate::connection::ConnectionError;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

impl Channel for Session {
    const NAME: &'static str = "session";
}
