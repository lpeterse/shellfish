mod map;
mod session;
mod type_;

use super::error::*;

pub use self::map::*;
pub use self::session::*;
pub use self::type_::*;

use std::sync::{Arc, Mutex};

pub struct Channel {
    is_closing: bool,
    local_channel: u32,
    local_window_size: u32,
    local_max_packet_size: u32,
    remote_channel: u32,
    remote_window_size: u32,
    remote_max_packet_size: u32,
    shared: SharedState,
}

impl Channel {
    pub fn new(
        local_channel: u32,
        local_window_size: u32,
        local_max_packet_size: u32,
        remote_channel: u32,
        remote_window_size: u32,
        remote_max_packet_size: u32,
        shared: SharedState,
    ) -> Self {
        Self {
            is_closing: false,
            local_channel,
            local_window_size,
            local_max_packet_size,
            remote_channel,
            remote_window_size,
            remote_max_packet_size,
            shared,
        }
    }

    pub fn is_closing(&self) -> bool {
        self.is_closing
    }

    pub fn local_channel(&self) -> u32 {
        self.local_channel
    }

    pub fn remote_channel(&self) -> u32 {
        self.remote_channel
    }

    pub fn shared(&self) -> &SharedState {
        &self.shared
    }

    pub fn decrease_local_window_size(&mut self, n: u32) -> Result<(), ConnectionError> {
        if n <= self.local_window_size {
            self.local_window_size -= n;
            return Ok(());
        }
        Err(ConnectionError::ChannelWindowSizeUnderflow)
    }

    pub fn increase_remote_window_size(&mut self, n: u32) -> Result<(), ConnectionError> {
        let n_: u64 = n as u64;
        let w_: u64 = self.remote_window_size as u64;
        if n_ + w_ <= (u32::max_value() as u64) {
            self.remote_window_size += n;
            return Ok(());
        }
        Err(ConnectionError::ChannelWindowSizeOverflow)
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        match &mut self.shared {
            SharedState::Session(x) => x.lock().unwrap().terminate(e),
        }
    }
}

pub enum SharedState {
    Session(Arc<Mutex<SessionState>>),
}

pub trait SpecificState {
    fn terminate(&mut self, e: ConnectionError);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_closing_01() {
        let c = Channel::new(
            0,
            0,
            0,
            0,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        assert_eq!(c.is_closing(), false);
    }

    #[test]
    fn test_local_channel_01() {
        let c = Channel::new(
            23,
            0,
            0,
            0,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        assert_eq!(c.local_channel(), 23);
    }

    #[test]
    fn test_remote_channel_01() {
        let c = Channel::new(
            0,
            0,
            0,
            23,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        assert_eq!(c.remote_channel(), 23);
    }

    #[test]
    fn test_shared_01() {
        let c = Channel::new(
            0,
            0,
            0,
            23,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        match c.shared() {
            SharedState::Session(_) => ()
        }
    }

    #[test]
    fn test_decrease_local_window_size_01() {
        let mut c = Channel::new(
            0,
            100,
            0,
            0,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        assert_eq!(c.decrease_local_window_size(50), Ok(()));
        assert_eq!(c.decrease_local_window_size(50), Ok(()));
        assert_eq!(
            c.decrease_local_window_size(50),
            Err(ConnectionError::ChannelWindowSizeUnderflow)
        );
    }

    #[test]
    fn test_increase_remote_window_size_01() {
        let mut c = Channel::new(
            0,
            0,
            0,
            0,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        assert_eq!(c.increase_remote_window_size(u32::max_value()), Ok(()));
        assert_eq!(
            c.increase_remote_window_size(1),
            Err(ConnectionError::ChannelWindowSizeOverflow)
        );
    }

    #[test]
    fn test_terminate_session_01() {
        let mut c = Channel::new(
            0,
            100,
            0,
            0,
            0,
            0,
            SharedState::Session(Arc::new(Mutex::new(Default::default()))),
        );
        let e = ConnectionError::ChannelWindowSizeUnderflow;
        match c.shared {
            SharedState::Session(ref s) => assert_eq!(s.lock().unwrap().inner_error, None),
        }
        c.terminate(e);
        match c.shared {
            SharedState::Session(ref s) => assert_eq!(s.lock().unwrap().inner_error, Some(e)),
        }
    }
}
