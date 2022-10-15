use super::channel::direct_tcpip::DirectTcpIpRequest;
use super::channel::session::SessionRequest;
use super::global::{GlobalRequest, GlobalRequestWantReply};
use super::{ConnectionError, DirectTcpIp};
use std::task::{Context, Poll};

pub trait ConnectionHandler: Send + Sync + 'static {
    fn on_request(&mut self, request: GlobalRequest) {
        log::error!("HANDLER ON_REQUEST: {:?}", request);
    }

    fn on_request_want_reply(&mut self, request: GlobalRequestWantReply) {
        log::error!("HANDLER ON_REQUEST_WANT_REPLY {:?}", request);
    }

    fn on_direct_tcpip_request(&mut self, request: DirectTcpIpRequest) {
        log::error!("HANDLER ON_DIRECT_TCPIP_REQUEST {:?}", request);
    }

    fn on_session_request(&mut self, request: SessionRequest) {
        log::error!("HANDLER ON_SESSION_REQUEST {:?}", request)
    }

    fn on_error(self: Box<Self>, e: &ConnectionError) {
        log::error!("HANDLER ON_ERROR: {}", e);
    }

    fn poll(&mut self, _cx: &mut Context) -> Poll<()> {
        Poll::Pending
    }
}

impl ConnectionHandler for () {}
