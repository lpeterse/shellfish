use super::channel::ChannelOpenRequest;
use super::global::{GlobalRequest, GlobalRequestWantReply, HostKeys};
use super::ConnectionError;
use crate::interpret;
use std::task::{Context, Poll};

pub trait ConnectionHandler: Send + Sync + 'static {
    fn on_request(&mut self, request: GlobalRequest) {
        log::error!("HANDLER ON_REQUEST: {:?}", request);
        interpret!(request, HostKeys, {
            log::error!("HANDLER ON_REQUEST: {:?}", request);
        });
        interpret!(request, (), {});
    }

    fn on_request_want_reply(&mut self, request: GlobalRequestWantReply) {
        log::error!("HANDLER ON_REQUEST_WANT_REPLY {:?}", request);
    }

    fn on_open_request(&mut self, request: ChannelOpenRequest) {
        log::error!("HANDLER ON_OPEN_REQUEST {:?}", request)
    }

    fn on_error(self: Box<Self>, e: &ConnectionError) {
        log::error!("HANDLER ON_ERROR: {}", e);
    }

    fn poll(&mut self, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }
}

impl ConnectionHandler for () {}
