/// Types and methods for interaction with `ssh-agent`s.
pub mod agent;
pub mod algorithm;
pub mod client;
pub mod host;
pub mod server;
pub mod service;
pub mod transport;

pub(crate) mod codec;
pub(crate) mod glob;
pub(crate) mod buffer;
pub(crate) mod role;
pub(crate) mod message;
pub(crate) mod util;
