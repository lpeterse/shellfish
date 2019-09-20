use crate::language::*;
use super::*;
use super::channel::*;

use futures::channel::oneshot;

pub enum Command {
    ChannelOpenSession(oneshot::Sender<Result<Channel<Session>,OpenFailure>>),
    Foobar
}
