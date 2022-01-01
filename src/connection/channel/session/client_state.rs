use super::{Process, PtySpecification, SessionClient};
use crate::connection::channel::RequestFailure;
use crate::connection::channel::{Channel, ChannelState, PollResult};
use crate::connection::error::ConnectionErrorWatch;
use crate::connection::msg::{
    MsgChannelClose, MsgChannelFailure, MsgChannelOpen, MsgChannelRequest,
};
use crate::connection::{ConnectionError, OpenFailure};
use crate::ready;
use crate::transport::Transport;
use crate::util::check;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot::{self, Sender};

// =================================================================================================
// CLIENT STATE 1 (Opening)
// =================================================================================================

#[derive(Debug)]
pub struct ClientState1 {
    lid: u32,
    lbs: u32,
    lps: u32,
    params: Option<()>,
    error_rx: ConnectionErrorWatch,
    reply_tx: Sender<Result<SessionClient, OpenFailure>>,
}

impl ClientState1 {
    pub fn new(
        lid: u32,
        lbs: u32,
        lps: u32,
        error_rx: ConnectionErrorWatch,
        reply_tx: Sender<Result<SessionClient, OpenFailure>>,
    ) -> Self {
        Self {
            lid,
            lbs,
            lps,
            params: Some(()),
            error_rx,
            reply_tx,
        }
    }

    fn msg_open(&self) -> MsgChannelOpen<&'static str> {
        MsgChannelOpen {
            name: SessionClient::NAME,
            sender_channel: self.lid,
            initial_window_size: self.lbs,
            maximum_packet_size: self.lps,
            data: vec![],
        }
    }
}

impl ChannelState for ClientState1 {
    fn on_open_confirmation(
        self: Box<Self>,
        rid: u32,
        rws: u32,
        rps: u32,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        let (req_tx, req_rx) = oneshot::channel();
        let s = ClientState2::new(self.lid, self.lbs, self.lps, rid, rws, rps, req_rx);
        let c = SessionClient::new(R1(req_tx), self.error_rx);
        let _ = self.reply_tx.send(Ok(c));
        Ok(Box::new(s))
    }

    fn on_open_failure(self: Box<Self>, e: OpenFailure) -> Result<(), ConnectionError> {
        let _ = self.reply_tx.send(Err(e));
        Ok(())
    }

    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        if self.params.is_some() {
            let msg = self.msg_open();
            ready!(t.poll_send(cx, &msg))?;
            self.params = None;
        }
        Poll::Ready(Ok(PollResult::Noop))
    }
}

// =================================================================================================
// CLIENT STATE 2 (Open, but no process started yet)
// =================================================================================================

#[derive(Debug)]
pub struct ClientState2 {
    lid: u32,
    lbs: u32,
    lps: u32,

    rid: u32,
    rws: u32,
    rps: u32,

    req_rcvd: usize,

    eof_rcvd: bool,

    close_send: bool,
    close_sent: bool,
    close_rcvd: bool,

    req_send_head: R2,
}

impl ClientState2 {
    pub(crate) fn new(
        lid: u32,
        lbs: u32,
        lps: u32,
        rid: u32,
        rws: u32,
        rps: u32,
        req_rx: oneshot::Receiver<R2>,
    ) -> Self {
        Self {
            lid,
            lbs,
            lps,
            rid,
            rws,
            rps,

            req_rcvd: 0,

            eof_rcvd: false,

            close_send: false,
            close_sent: false,
            close_rcvd: false,

            req_send_head: panic!(),
        }
    }

    // Send outstanding env request (if any)
    fn poll_send_req(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        // ) = self.req_send_head.open {
        //     match o {
        //         R3::Env {
        //             params: Some(p),
        //             reply,
        //         } => {
        //             let msg = MsgChannelRequest {
        //                 recipient_channel: self.rid,
        //                 request: "env",
        //                 want_reply: reply.is_some(),
        //                 specific: (p.0.as_str(), p.1.as_str()),
        //             };
        //             ready!(t.poll_send(cx, &msg))?;
        //             if reply.is_none() {
        //                 self.req_send_head.open = None
        //             } else {
        //                 o.set_sent()
        //             }
        //         }
        //         _ => (),
        //     }
        // }
        panic!()
    }

    fn poll_send_rej(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        panic!()
    }
}

impl ChannelState for ClientState2 {
    fn on_window_adjust(&mut self, bytes: u32) -> Result<(), ConnectionError> {
        check(!self.close_rcvd).ok_or(ConnectionError::ChannelWindowAdjustUnexpected)?;
        check(self.rws + bytes > bytes).ok_or(ConnectionError::ChannelWindowAdjustOverflow)?;
        self.rws += bytes;
        Ok(())
    }

    fn on_eof(&mut self) -> Result<(), ConnectionError> {
        check(!self.eof_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        check(!self.close_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        self.eof_rcvd = true;
        Ok(())
    }

    fn on_close(&mut self) -> Result<(), ConnectionError> {
        check(!self.close_rcvd).ok_or(ConnectionError::ChannelCloseUnexpected)?;
        self.close_rcvd = true;
        self.close_send = true;
        Ok(())
    }

    fn on_request(
        &mut self,
        _name: &str,
        _data: &[u8],
        want_reply: bool,
    ) -> Result<(), ConnectionError> {
        check(!self.close_rcvd).ok_or(ConnectionError::ChannelRequestUnexpected)?;
        self.req_rcvd += want_reply as usize;
        Ok(())
    }

    fn on_success(mut self: Box<Self>) -> Result<Box<dyn ChannelState>, ConnectionError> {
        let e = ConnectionError::ChannelSuccessUnexpected;
        match self.req_send_head.open.reply.take() {
            Some(SessionRes2::Unit(reply)) => {
                let _ = reply.send(Ok(()));
                Ok(self)
            }
            Some(SessionRes2::Proc(reply)) => {
                let state = Box::new(ClientState3::from());
                let process = todo!("PROCESS");
                let _ = reply.send(Ok(process));
                Ok(state)
            }
            _ => Err(e),
        }
    }

    fn on_failure(&mut self) -> Result<(), ConnectionError> {
        let e = ConnectionError::ChannelFailureUnexpected;
        let f = RequestFailure(());
        match self.req_send_head.open.reply.take() {
            Some(SessionRes2::Unit(reply)) => {
                let _ = reply.send(Err(f));
                Ok(())
            }
            Some(SessionRes2::Proc(reply)) => {
                let _ = reply.send(Err(f));
                Ok(())
            }
            _ => Err(e),
        }
    }

    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        loop {
            if self.close_send {
                if !self.close_sent {
                    let msg = MsgChannelClose::new(self.rid);
                    ready!(t.poll_send(cx, &msg))?;
                    self.close_sent = true;
                }
                return if self.close_rcvd {
                    Poll::Ready(Ok(PollResult::Closed))
                } else {
                    Poll::Ready(Ok(PollResult::Noop))
                };
            }

            while self.req_rcvd > 0 {
                let msg = MsgChannelFailure::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
            }

            if self.req_send_head.open.param.is_none() && self.req_send_head.open.reply.is_none() {
                match Pin::new(&mut self.req_send_head.next).poll(cx) {
                    Poll::Ready(Ok(x)) => self.req_send_head = x,
                    Poll::Ready(Err(_)) => self.close_send = true,
                    Poll::Pending => return Poll::Ready(Ok(PollResult::Noop)),
                }
            }

            if let Some(param) = &self.req_send_head.open.param {
                let want_reply = self.req_send_head.open.reply.is_some();
                match param {
                    SessionReq2::Env((k, v)) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: self.rid,
                            request: "env",
                            want_reply,
                            specific: (k.as_str(), v.as_str()),
                        };
                        ready!(t.poll_send(cx, &msg))?;
                    }
                    SessionReq2::Pty(ref pty) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: self.rid,
                            request: "pty-req",
                            want_reply,
                            specific: pty,
                        };
                        ready!(t.poll_send(cx, &msg))?;
                    }
                    SessionReq2::Shell => {
                        let msg = MsgChannelRequest {
                            recipient_channel: self.rid,
                            request: "shell",
                            want_reply,
                            specific: (),
                        };
                        ready!(t.poll_send(cx, &msg))?;
                    }
                    SessionReq2::Exec(cmd) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: self.rid,
                            request: "exec",
                            want_reply,
                            specific: cmd.as_str(),
                        };
                        ready!(t.poll_send(cx, &msg))?;
                    }
                    SessionReq2::Subsystem(cmd) => {
                        let msg = MsgChannelRequest {
                            recipient_channel: self.rid,
                            request: "subsystem",
                            want_reply,
                            specific: cmd.as_str(),
                        };
                        ready!(t.poll_send(cx, &msg))?;
                    }
                }
                self.req_send_head.open.param = None;
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct R1(oneshot::Sender<R2>);

impl R1 {
    pub fn req_unit(
        &mut self,
        param: SessionReq2,
        want_reply: bool,
    ) -> oneshot::Receiver<Result<(), RequestFailure>> {
        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        let Self(tx) = std::mem::replace(self, Self(tx1));
        let r4 = R4 {
            param: Some(param),
            reply: Some(SessionRes2::Unit(tx2)),
        };
        let r2 = R2 {
            next: rx1,
            open: r4,
        };
        let _ = tx.send(r2);
        rx2
    }

    pub fn req_proc(
        &mut self,
        param: SessionReq2,
    ) -> oneshot::Receiver<Result<Process, RequestFailure>> {
        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
        let Self(tx) = std::mem::replace(self, Self(tx1));
        let r4 = R4 {
            param: Some(param),
            reply: Some(SessionRes2::Proc(tx2)),
        };
        let r2 = R2 {
            next: rx1,
            open: r4,
        };
        let _ = tx.send(r2);
        rx2
    }
}

#[derive(Debug)]
pub(crate) struct R2 {
    next: oneshot::Receiver<R2>,
    open: R4,
}

#[derive(Debug, Default)]
pub(crate) struct R4 {
    param: Option<SessionReq2>,
    reply: Option<SessionRes2>,
}

#[derive(Debug)]
pub enum SessionRes2 {
    Unit(oneshot::Sender<Result<(), RequestFailure>>),
    Proc(oneshot::Sender<Result<Process, RequestFailure>>),
}

#[derive(Debug)]
pub enum SessionReq2 {
    Env((String, String)),
    Pty(PtySpecification),
    Shell,
    Exec(String),
    Subsystem(String),
}

#[derive(Debug)]
pub enum SessionRun {
    Shell,
    Exec(String),
    Subsystem(String),
}

// =================================================================================================
// CLIENT STATE 3 (Open, process started)
// =================================================================================================

#[derive(Debug)]
pub(crate) struct ClientState3;

impl ClientState3 {
    pub fn from() -> Self {
        panic!()
    }
}

impl ChannelState for ClientState3 {}
