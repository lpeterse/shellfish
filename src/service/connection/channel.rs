mod map;
mod session;
mod type_;

use super::error::*;

pub use self::map::*;
pub use self::session::*;
pub use self::type_::*;

use std::sync::{Arc, Mutex};

pub struct Channel {
    pub is_closing: bool,
    pub local_channel: u32,
    pub local_window_size: u32,
    pub local_max_packet_size: u32,
    pub remote_channel: u32,
    pub remote_window_size: u32,
    pub remote_max_packet_size: u32,
    pub shared: SharedState,
}

impl Channel {
    pub fn decrease_local_window_size(&mut self, n: usize) -> Result<(), ConnectionError> {
        if (n as u32) > self.local_window_size {
            Err(ConnectionError::ChannelWindowSizeUnderrun)
        } else {
            self.local_window_size -= n as u32;
            Ok(())
        }
    }
}

pub enum SharedState {
    Session(Arc<Mutex<SessionState>>),
}

pub trait SpecificState {
    fn terminate(&mut self, e: ConnectionError);
}

impl Channel {
    pub fn terminate(&mut self, e: ConnectionError) {
        match &mut self.shared {
            SharedState::Session(x) => x.lock().unwrap().terminate(e),
        }
    }
}
