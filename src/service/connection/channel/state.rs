use super::super::error::*;

pub use super::type_::*;
pub use super::session::*;
pub use super::other::*;
pub use super::request::*;

use std::sync::{Arc,Mutex};
use futures::task::{AtomicWaker};

pub struct ChannelState {
    pub is_closing: bool,
    pub local_channel: u32,
    pub local_window_size: u32,
    pub local_max_packet_size: u32,
    pub remote_channel: u32,
    pub remote_window_size: u32,
    pub remote_max_packet_size: u32,
    pub shared: TypedState,
}

pub enum TypedState {
    Session(Arc<Mutex<SharedState<Session>>>)
}

pub struct SharedState<T: ChannelType> {
    pub connection_task: AtomicWaker,
    pub connection_error: Option<ConnectionError>,
    pub user_task: AtomicWaker,
    pub user_error: Option<()>,
    pub specific: T::SpecificState,
}

impl <T: ChannelType> SharedState<T> {
    pub fn terminate_as_connection(&mut self, e: ConnectionError) {
        self.connection_error = Some(e);
        self.user_task.wake();
    }

    pub fn terminate_as_user(&mut self) {
        self.user_error = Some(());
        self.connection_task.wake();
    }
}

impl <T: ChannelType> Default for SharedState<T> {
    fn default() -> Self {
        SharedState {
            connection_task: AtomicWaker::new(),
            connection_error: None,
            user_task: AtomicWaker::new(),
            user_error: None,
            specific: Default::default(),
        }
    }
}

impl ChannelState {
    pub fn terminate(self, e: ConnectionError) {
        match self.shared {
            TypedState::Session(st) => st.lock().unwrap().terminate_as_connection(e)
        }
    }
}
