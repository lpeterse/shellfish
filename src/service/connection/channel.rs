mod type_;

pub use self::type_::*;
use super::msg_channel_open_failure::ChannelOpenFailureReason;
use crate::language::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use std::collections::VecDeque;

pub struct Channel<T: ChannelType> {
    pub id: u32,
    pub request: <T as ChannelType>::Request,
    pub confirmation: <T as ChannelType>::Confirmation,
    pub notification: mpsc::Receiver<()>,
}

pub enum ChannelState {
    Opening(oneshot::Sender<Result<Channel<Session>,OpenFailure>>),
    Open(Open),
    Closing,
}

pub struct Open {
    pub local_channel: u32,
    pub local_initial_window_size: u32,
    pub local_max_packet_size: u32,
    pub remote_channel: u32,
    pub remote_initial_window_size: u32,
    pub remote_max_packet_size: u32,
    pub receive_buffer: VecDeque<u8>,
    pub notify: mpsc::Sender<()>,
}

pub struct OpenFailure {
    pub reason: ChannelOpenFailureReason,
    pub description: String,
}

pub struct OpenFailureReason {
}

pub struct ChannelOpenConfirmation<T: ChannelType> {
    pub id: u32,
    pub specific: T::Confirmation
}

#[derive(Debug)]
pub struct ChannelOpenFailure {
    pub reason_code: u32,
    pub description: String,
    pub language: Language,
}

