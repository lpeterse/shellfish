mod debug;
mod disconnect;
mod ecdh_init;
mod ecdh_reply;
mod ignore;
mod kex_init;
mod new_keys;
mod service_accept;
mod service_request;
mod unimplemented;

pub use debug::*;
pub use disconnect::*;
pub use ecdh_init::*;
pub use ecdh_reply::*;
pub use ignore::*;
pub use kex_init::*;
pub use new_keys::*;
pub use service_accept::*;
pub use service_request::*;
pub use unimplemented::*;

/// All types representing SSH_MSG_* messages shall implement this trait.
pub trait Message {
    // The message number as speicified in the RFCs.
    const NUMBER: u8;
}
