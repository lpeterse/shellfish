use super::*;

#[derive(Debug)]
pub enum ConnectionRequest {
    Global(GlobalRequest),
    ChannelOpen(ChannelOpenRequest),
}
