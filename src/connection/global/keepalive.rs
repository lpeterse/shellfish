use super::*;

#[derive(Debug)]
pub struct KeepAlive;

impl Global for KeepAlive {
    const NAME: &'static str = "keepalive@openssh.com";
    type RequestData = ();
}

impl GlobalWantReply for KeepAlive {
    type ResponseData = ();
}
