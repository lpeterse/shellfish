mod type_;

use super::*;
pub use self::type_::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use std::collections::VecDeque;

pub struct Channel<T: ChannelType> {
    pub id: u32,
    pub request: <T as ChannelType>::Request,
    pub confirmation: <T as ChannelType>::Confirmation,
}

pub enum ChannelState {
    Opening(OpeningChannel),
    Open(OpenChannel),
    Closing(ClosingChannel),
}

pub struct OpeningChannel {
    pub notify: oneshot::Sender<()>,
}

pub struct OpenChannel {
    pub local_channel: u32,
    pub local_initial_window_size: u32,
    pub local_max_packet_size: u32,
    pub remote_channel: u32,
    pub remote_initial_window_size: u32,
    pub remote_max_packet_size: u32,
    pub receive_buffer: VecDeque<u8>,
    pub notify: mpsc::Sender<()>,
}

pub struct ClosingChannel {
    pub notify: oneshot::Sender<()>,
}
