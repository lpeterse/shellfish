#![feature(process_exitcode_placeholder)]
#![feature(todo_macro)]

pub mod algorithm;
pub mod agent;
pub mod client;
pub mod service;
pub mod transport;

pub(crate) mod util;
pub(crate) mod codec;
pub(crate) mod ring_buffer;
pub(crate) mod role;
pub(crate) mod message;
