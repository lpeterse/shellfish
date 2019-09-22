use crate::language::*;
use super::*;
use super::channel::*;

use futures::channel::oneshot;

pub enum Command {
    Debug(String),
    Disconnect,
    ChannelOpenSession(oneshot::Sender<Result<Channel<Session>,OpenFailure>>),
}
