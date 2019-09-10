use crate::language::*;
use super::*;

use futures::channel::oneshot;
use futures::channel::mpsc;
use futures::future::TryFutureExt;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use async_std::task;

pub enum Command {
    ChannelOpenSession(ChannelOpen<Session>)
}

pub struct ChannelOpen<T: ChannelType> {
    pub result: oneshot::Sender<Result<ChannelOpenConfirmation<T>,ChannelOpenFailure>>
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
