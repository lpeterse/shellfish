use super::super::super::error::ConnectionError;
use super::super::super::msg::*;
use super::super::open_failure::OpenFailure;
use super::super::state::ChannelState;
use super::DirectTcpIp;

use crate::ready;
use crate::transport::GenericTransport;
use crate::util::buffer::Buffer;
use crate::util::check;
use std::future::Future;
use std::mem::replace;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use tokio::sync::oneshot;

pub const EV_FLUSHED: u8 = 1;
pub const EV_READABLE: u8 = 2;
pub const EV_WRITABLE: u8 = 4;
pub const EV_EOF_SENT: u8 = 8;
pub const EV_EOF_RCVD: u8 = 16;
pub const EV_CLOSE_RCVD: u8 = 64;
pub const EV_ANY: u8 = 255;

#[derive(Debug)]
pub enum Status {
    OpeningAwaitConfirm(oneshot::Sender<Result<DirectTcpIp, OpenFailure>>),
    OpeningAwaitDecision(oneshot::Receiver<Result<(), OpenFailure>>),
    OpeningAwaitTransmission(Result<(), OpenFailure>),
    Open,
    Closed,
    Error(Arc<ConnectionError>),
}

#[derive(Debug)]
pub struct DirectTcpIpState {
    pub mbs: usize,

    pub lid: u32,
    pub lws: u32,
    pub lmps: u32,

    pub rid: u32,
    pub rws: u32,
    pub rmps: u32,

    pub eof: bool,
    pub eof_sent: bool,
    pub eof_rcvd: bool,

    pub close: bool,
    pub close_sent: bool,
    pub close_rcvd: bool,

    pub stdin: Buffer,
    pub stdout: Buffer,

    pub inner_task_waker: Option<Waker>,
    pub outer_task_waker: Option<Waker>,
    pub outer_task_flags: u8,

    pub status: Status,
}

impl DirectTcpIpState {
    pub fn new_outbound(
        lid: u32,
        lws: u32,
        lps: u32,
        resp: oneshot::Sender<Result<DirectTcpIp, OpenFailure>>,
    ) -> Self {
        Self {
            mbs: lws as usize,

            lid,
            lws,
            lmps: lps,

            rid: 0,
            rws: 0,
            rmps: 0,

            eof: false,
            eof_sent: false,
            eof_rcvd: false,

            close: false,
            close_sent: false,
            close_rcvd: false,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0),

            inner_task_waker: None,
            outer_task_waker: None,
            outer_task_flags: 0,

            status: Status::OpeningAwaitConfirm(resp),
        }
    }

    pub fn new_inbound(
        lid: u32,
        mbs: u32,
        lmps: u32,
        rid: u32,
        rws: u32,
        rmps: u32,
        resp: oneshot::Receiver<Result<(), OpenFailure>>,
    ) -> Self {
        Self {
            mbs: mbs as usize,

            lid,
            lws: mbs,
            lmps,

            rid,
            rws,
            rmps,

            eof: false,
            eof_sent: false,
            eof_rcvd: false,

            close: false,
            close_sent: false,
            close_rcvd: false,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0),

            inner_task_waker: None,
            outer_task_waker: None,
            outer_task_flags: 0,

            status: Status::OpeningAwaitDecision(resp),
        }
    }

    pub fn push_open_confirmation(
        &mut self,
        rid: u32,
        rws: u32,
        rps: u32,
        handle: DirectTcpIp,
    ) -> Result<(), ConnectionError> {
        if let Status::OpeningAwaitConfirm(x) = replace(&mut self.status, Status::Open) {
            self.rid = rid;
            self.rws = rws;
            self.rmps = rps;
            self.close = x.send(Ok(handle)).is_err();
            Ok(())
        } else {
            Err(ConnectionError::ChannelOpenConfirmationUnexpected)?
        }
    }

    pub fn push_open_failure(&mut self, reason: OpenFailure) -> Result<(), ConnectionError> {
        if let Status::OpeningAwaitConfirm(x) = replace(&mut self.status, Status::Closed) {
            let _ = x.send(Err(reason));
            Ok(())
        } else {
            Err(ConnectionError::OpenFailureUnexpected)?
        }
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let len = data.len() as u32;
        check(!self.eof && !self.close).ok_or(ConnectionError::ChannelDataUnexpected)?;
        check(len <= self.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
        check(len <= self.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
        self.lws -= len;
        self.stdin.write_all(data);
        Ok(())
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        /*
        match self.ext {
            Some(ref mut ext) if code == SSH_EXTENDED_DATA_STDERR && !self.eof && !self.close => {
                let len = data.len() as u32;
                check(len <= self.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
                check(len <= self.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
                self.lws -= len;
                ext.rx.write_all(data);
                Ok(())
            }
            _ => Err(ConnectionError::ChannelExtendedDataUnexpected),
        }*/
        panic!()
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        check(!self.close).ok_or(ConnectionError::ChannelWindowAdjustUnexpected)?;
        if (n as u64 + self.rws as u64) > (u32::MAX as u64) {
            return Err(ConnectionError::ChannelWindowAdjustOverflow);
        }
        self.rws += n;
        Ok(())
    }

    pub fn push_request(
        &mut self,
        _name: &str,
        _data: &[u8],
        _want_reply: bool,
    ) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_success(&mut self) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_failure(&mut self) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        check(!self.eof && !self.close).ok_or(ConnectionError::ChannelEofUnexpected)?;
        self.eof_rcvd = true;
        Ok(())
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        check(!self.close).ok_or(ConnectionError::ChannelCloseUnexpected)?;
        self.close = true;
        self.close_rcvd = true;
        Ok(())
    }

    pub fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut GenericTransport,
    ) -> Poll<Result<(), ConnectionError>> {
        loop {
            match &mut self.status {
                Status::OpeningAwaitDecision(ref mut x) => {
                    let e = Err(OpenFailure::ADMINISTRATIVELY_PROHIBITED);
                    let r = match Future::poll(Pin::new(x), cx) {
                        Poll::Pending => return Poll::Ready(Ok(())),
                        Poll::Ready(r) => r.unwrap_or(e),
                    };
                    self.status = Status::OpeningAwaitTransmission(r);
                    continue;
                }
                Status::OpeningAwaitTransmission(Ok(())) => {
                    let msg = MsgChannelOpenConfirmation {
                        recipient_channel: self.rid,
                        sender_channel: self.lid,
                        initial_window_size: self.lws,
                        maximum_packet_size: self.lmps,
                        specific: b"",
                    };
                    ready!(t.poll_send(cx, &msg))?;
                    self.status = Status::Open;
                    continue;
                }
                Status::OpeningAwaitTransmission(Err(r)) => {
                    let msg = MsgOpenFailure::new(self.rid, *r);
                    ready!(t.poll_send(cx, &msg))?;
                    self.status = Status::Closed;
                    continue;
                }
                Status::Open if self.inner_task_waker.is_none() => {
                    // Send data as long as data is available or the remote window size
                    while !self.stdout.is_empty() {
                        let len = std::cmp::min(self.rmps as usize, self.stdout.len());
                        let dat = &self.stdout.as_ref()[..len];
                        let msg = MsgChannelData::new(self.rid, dat);
                        ready!(t.poll_send(cx, &msg))?;
                        log::debug!("#{}: Sent MSG_CHANNEL_DATA ({})", self.lid, len);
                        self.stdout.consume(len);
                    }
                    // Send eof if flag set and eof not yet sent.
                    if self.eof && !self.eof_sent {
                        let msg = MsgChannelEof::new(self.rid);
                        ready!(t.poll_send(cx, &msg))?;
                        log::debug!("#{}: Sent MSG_CHANNEL_EOF", self.lid);
                        self.eof_sent = true;
                    }
                    // Send close if flag set and close not yet sent.
                    if self.close && !self.close_sent {
                        let msg = MsgChannelClose::new(self.rid);
                        ready!(t.poll_send(cx, &msg))?;
                        log::debug!("#{}: Sent MSG_CHANNEL_CLOSE", self.lid);
                        self.close_sent = true;
                    }
                    // Send window adjust message when threshold is reached.
                    if let Some(n) = self.recommended_window_adjust() {
                        let msg = MsgChannelWindowAdjust::new(self.rid, n);
                        ready!(t.poll_send(cx, &msg))?;
                        log::debug!("#{}: Sent MSG_CHANNEL_WINDOW_ADJUST ({})", self.lid, n);
                        self.lws += n;
                    }
                    self.inner_task_waker = Some(cx.waker().clone());
                    break;
                }
                _ => break,
            }
        }
        Poll::Ready(Ok(()))
    }

    pub fn recommended_window_adjust(&mut self) -> Option<u32> {
        let threshold = self.mbs / 2;
        if (self.lws as usize) < threshold {
            let buffered = self.stdin.len();
            if buffered < threshold {
                let adjustment = self.mbs - std::cmp::max(self.lws as usize, buffered);
                return Some(adjustment as u32);
            }
        }
        None
    }

    pub fn take_outer_waker(&mut self, flags: u8) -> Option<Waker> {
        if self.outer_task_flags & flags != 0 {
            self.outer_task_flags = 0;
            self.outer_task_waker.take()
        } else {
            None
        }
    }
}

impl ChannelState for Arc<Mutex<DirectTcpIpState>> {
    fn on_open_confirmation(
        self: Box<Self>,
        rid: u32,
        rws: u32,
        rps: u32,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        let st = self.as_ref().clone();
        let mut ch = self.lock().unwrap();
        if let Status::OpeningAwaitConfirm(x) = replace(&mut ch.status, Status::Open) {
            ch.rid = rid;
            ch.rws = rws;
            ch.rmps = rps;
            ch.close = x.send(Ok(DirectTcpIp(st))).is_err();
            drop(ch);
            Ok(self)
        } else {
            Err(ConnectionError::ChannelOpenConfirmationUnexpected)?
        }
    }

    fn on_open_failure(self: Box<Self>, e: OpenFailure) -> Result<(), ConnectionError> {
        let mut ch = self.lock().unwrap();
        if let Status::OpeningAwaitConfirm(x) = replace(&mut ch.status, Status::Closed) {
            let _ = x.send(Err(e));
            Ok(())
        } else {
            Err(ConnectionError::OpenFailureUnexpected)?
        }
    }

    fn on_error(self: Box<Self>, e: &Arc<ConnectionError>) {
        drop(e);
        todo!()
    }

    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut GenericTransport,
    ) -> Poll<Result<bool, ConnectionError>> {
        todo!()
    }
}
