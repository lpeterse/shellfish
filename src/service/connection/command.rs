use crate::language::*;
use super::*;

use futures::channel::oneshot;

pub enum Command {
    ChannelOpenSession(ChannelOpen<Session>),
    Foobar
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

