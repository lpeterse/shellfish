use super::channel::ChannelHandle;
use super::channel::ChannelOpenFailure;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum ConnectionRequest {
    Global {
        name: &'static str,
        data: Vec<u8>,
        reply: Option<oneshot::Sender<Result<Vec<u8>, ()>>>,
    },
    Open {
        name: &'static str,
        data: Vec<u8>,
        reply: oneshot::Sender<Result<ChannelHandle, ChannelOpenFailure>>,
    },
}
