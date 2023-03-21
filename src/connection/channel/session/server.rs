use super::super::super::config::ConnectionConfig;
use super::super::super::error::ConnectionError;
use super::super::super::msg::*;
use super::super::ChannelState;
use super::{Process, Exit};
use crate::connection::channel::PollResult;
use crate::connection::OpenFailure;
use crate::transport::Transport;
use crate::util::buffer::Buffer;
use crate::util::check;
use crate::util::codec::SshCodec;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tokio::io::ReadBuf;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct SessionServerState {
    lid: u32,
    lws: u32,
    lps: u32,
    rid: u32,
    rws: u32,
    rps: u32,
    mbs: u32,

    stdin: Buffer,
    stdout: Buffer,
    stderr: Buffer,
    replies: VecDeque<bool>,

    is_open: bool,
    is_stdin_broken: bool,
    is_stdout_eof: bool,
    is_stdout_broken: bool,
    is_stderr_eof: bool,
    is_stderr_broken: bool,
    is_eof_rcvd: bool,
    is_eof_sent: bool,
    is_close_rcvd: bool,
    is_close_sent: bool,

    exit: Option<Exit>,
    exit_sent: bool,

    open_req: Option<oneshot::Receiver<Result<Box<dyn SessionHandler>, OpenFailure>>>,
    open_res: Option<Result<(), OpenFailure>>,
    handler: Option<Box<dyn SessionHandler>>,
    process: Option<Box<dyn Process>>,
}

impl SessionServerState {
    pub(crate) fn open(
        config: &ConnectionConfig,
        msg: &MsgChannelOpen,
        lid: u32,
    ) -> Result<(Box<dyn ChannelState>, SessionRequest), ConnectionError> {
        let (req_tx, req_rx) = oneshot::channel();
        let cst = Self {
            lid,
            lws: config.channel_max_buffer_size,
            lps: config.channel_max_packet_size,
            rid: msg.sender_channel,
            rws: msg.initial_window_size,
            rps: msg.maximum_packet_size,
            mbs: config.channel_max_buffer_size,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0), // TODO: use transport as buffer
            stderr: Buffer::new(0), // TODO: use transport as buffer
            replies: VecDeque::new(),

            is_open: false,
            is_stdin_broken: false,
            is_stdout_eof: false,
            is_stdout_broken: false,
            is_stderr_eof: false,
            is_stderr_broken: false,
            is_eof_rcvd: false,
            is_eof_sent: false,
            is_close_rcvd: false,
            is_close_sent: false,

            exit: None,
            exit_sent: false,

            open_req: Some(req_rx),
            open_res: None,
            handler: None,
            process: None,
        };
        let cst = Box::new(cst);
        let req = SessionRequest { tx: req_tx };
        Ok((cst, req))
    }

    fn poll_open(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        if let Some(req) = &mut self.open_req {
            let res = ready!(Future::poll(Pin::new(req), cx));
            let res = res.unwrap_or(Err(OpenFailure::ADMINISTRATIVELY_PROHIBITED));
            let res = res.map(|x| self.handler = Some(x));
            self.open_req = None;
            self.open_res = Some(res);
        }

        if let Some(res) = &self.open_res {
            match res {
                Ok(()) => {
                    let rid = self.rid;
                    let lid = self.lid;
                    let lws = self.lws;
                    let lps = self.lps;
                    let msg = MsgChannelOpenConfirmation::new(rid, lid, lws, lps);
                    ready!(t.poll_send(cx, &msg))?;
                    self.open_res = None;
                    self.is_open = true;
                    let mps = std::cmp::min(self.rps as usize, self.lps as usize);
                    self.stdout.increase_capacity(mps);
                    self.stderr.increase_capacity(mps);
                }
                Err(e) => {
                    let rid = self.rid;
                    let msg = MsgChannelOpenFailure::new(rid, *e);
                    ready!(t.poll_send(cx, &msg))?;
                    self.open_res = None;
                    self.is_close_rcvd = true;
                    self.is_close_sent = true;
                }
            }
        }

        Poll::Ready(Ok(()))
    }

    fn poll_window_adjust(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        let e = ConnectionError::ChannelWindowSizeOverflow;
        let used = self.stdin.len() as u64 + self.lws as u64;
        let used = u32::try_from(used).ok().ok_or(e)?;

        if used <= self.mbs / 2 {
            let rid = self.rid;
            let inc = self.mbs - used;
            let msg = MsgChannelWindowAdjust::new(rid, inc);
            ready!(t.poll_send(cx, &msg))?;
            self.lws += inc;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_replies(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        while let Some(success) = self.replies.front() {
            if *success {
                let msg = MsgChannelSuccess::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
                let _ = self.replies.pop_front();
            } else {
                let msg = MsgChannelFailure::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
                let _ = self.replies.pop_front();
            }
        }

        Poll::Ready(Ok(()))
    }

    fn poll_stdin(&mut self, cx: &mut Context) -> Poll<Result<(), ConnectionError>> {
        if let Some(proc) = self.process.as_mut() {
            let mut dirty = false;
            while !self.is_stdin_broken && !self.stdin.is_empty() {
                match proc.poll_stdin_write(cx, self.stdin.as_ref()) {
                    Poll::Ready(Ok(len)) => {
                        self.stdin.consume(len);
                        dirty = true;
                    }
                    Poll::Ready(Err(_)) => {
                        self.is_stdin_broken = true;
                        break;
                    }
                    Poll::Pending => break,
                }
            }
            if !self.is_stdin_broken && dirty {
                if let Poll::Ready(Err(_)) = proc.poll_stdin_flush(cx) {
                    self.is_stdin_broken = true
                }
            }
        }

        Poll::Ready(Ok(()))
    }

    fn poll_stderr_stdout(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        if let Some(proc) = self.process.as_mut() {
            while self.rws > 0 {
                // Send from internal buffer
                macro_rules! xxx {
                    ($buf:expr,$f:expr) => {
                        if !$buf.is_empty() {
                            let len = $buf.len();
                            let len = len.min(self.rps as usize);
                            let len = len.min(self.rws as usize);
                            let dat = &$buf.as_ref()[..len];
                            let msg = $f(dat);
                            ready!(t.poll_send(cx, &msg)?);
                            $buf.consume(len);
                            self.rws -= len as u32;
                            continue
                        }
                    }
                }
                xxx!{self.stderr, |dat| MsgChannelExtendedData::new(self.rid, 1, dat)};
                xxx!{self.stdout, |dat| MsgChannelData::new(self.rid, dat)};
                // Read from process into intermediate buffer
                macro_rules! yyy {
                    ($buf:ident, $is_eof:ident, $is_broken:ident, $poll_read:ident) => {
                        if !self.$is_eof && !self.$is_broken {
                            assert!(self.$buf.is_empty());
                            let mut buf = ReadBuf::new(self.$buf.available_mut());
                            match proc.$poll_read(cx, &mut buf) {
                                Poll::Ready(Ok(())) if !buf.filled().is_empty() => {
                                    let len = buf.filled().len();
                                    self.$buf.extend(len);
                                    continue;
                                }
                                Poll::Ready(Ok(())) => {
                                    self.$is_eof = true;
                                }
                                Poll::Ready(Err(_)) => {
                                    self.$is_broken = true;
                                }
                                Poll::Pending => {
                                    ()
                                }
                            }
                        }
                    }
                }
                yyy!(stderr, is_stderr_eof, is_stderr_broken, poll_stderr_read);
                yyy!(stdout, is_stdout_eof, is_stdout_broken, poll_stdout_read);
                break
            }

            if (self.is_stdout_eof && self.is_stderr_eof) && !self.is_eof_sent {
                let msg = MsgChannelEof::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
                self.is_eof_sent = true;
            }
        }

        Poll::Ready(Ok(()))
    }

    /// Poll sending an exit message (if appropriate)
    /// 
    /// If the process terminated the function tries to send an exit message.
    /// It returns `Pending` only if the transport is blocking.
    fn poll_exit(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<(), ConnectionError>> {
        if let Some(proc) = self.process.as_mut() {
            if self.exit.is_none() {
                if let Poll::Ready(exit) = proc.poll_exit_status(cx) {
                    self.exit = Some(exit)
                }
            }
        }

        if let Some(exit) = self.exit.as_ref() {
            if !self.exit_sent {
                match exit {
                    Exit::Status(status) => {
                        let msg = MsgChannelRequest::new_exit_status(self.rid, status);
                        ready!(t.poll_send(cx, &msg))?;
                        self.exit_sent = true;
                    }
                    Exit::Signal(signal) => {
                        let msg = MsgChannelRequest::new_exit_signal(self.rid, signal);
                        ready!(t.poll_send(cx, &msg))?;
                        self.exit_sent = true;
                    }
                }
            }
        }

        Poll::Ready(Ok(()))
    }

    /// Poll sending a close message (if appropriate)
    /// 
    /// The channel shall be closed if either the peer requests it to be closed or if the process
    /// terminated and all outstanding data has been sent (eof).
    /// 
    /// The function determines if a close message needs to be sent and if so tries to send it.
    /// It returns `Pending` only if the transport is blocking.
    fn poll_close(
        &mut self,
        cx: &mut Context,
        t: &mut Transport
    ) -> Poll<Result<(), ConnectionError>> {
        let is_close_suitable = self.is_eof_sent && self.exit_sent;
        let is_close_required = is_close_suitable || self.is_close_rcvd;

        if is_close_required && !self.is_close_sent {
            let msg = MsgChannelClose::new(self.rid);
            ready!(t.poll_send(cx, &msg))?;
            self.is_close_sent = true;
        }

        Poll::Ready(Ok(()))
    }
}

impl ChannelState for SessionServerState {
    fn on_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        check(self.lws as usize >= data.len()).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
        self.stdin.write_all(data);
        self.lws -= data.len() as u32;
        Ok(())
    }

    fn on_window_adjust(&mut self, bytes: u32) -> Result<(), ConnectionError> {
        let rws = self.rws as u64 + bytes as u64;
        check(rws <= u32::MAX as u64).ok_or(ConnectionError::ChannelWindowSizeOverflow)?;
        self.rws = rws as u32;
        Ok(())
    }

    fn on_eof(&mut self) -> Result<(), ConnectionError> {
        check(!self.is_eof_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        check(!self.is_close_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        self.is_eof_rcvd = true;
        Ok(())
    }

    fn on_close(&mut self) -> Result<(), ConnectionError> {
        check(!self.is_close_rcvd).ok_or(ConnectionError::ChannelEofUnexpected)?;
        self.is_close_rcvd = true;
        Ok(())
    }

    fn on_request(
        &mut self,
        name: &str,
        data: &[u8],
        want_reply: bool,
    ) -> Result<(), ConnectionError> {
        macro_rules! reply {
            ($success:expr) => {
                if want_reply {
                    self.replies.push_back($success);
                }
            };
        }
        match name {
            "env" => {
                let h = self.handler.as_mut();
                let (key, val) = SshCodec::decode(data)?;
                reply!(h.and_then(|h| h.on_env_request(key, val)).is_some());
            }
            "pty-req" => {
                let h = self.handler.as_mut();
                reply!(h.and_then(|h| h.on_pty_request()).is_some());
            }
            "shell" => {
                if self.process.is_some() {
                    reply!(false);
                } else {
                    let handler = self.handler.take().unwrap();
                    self.process = handler.on_shell_request();
                    reply!(self.process.is_some());
                }
            }
            "exec" => {
                if self.process.is_some() {
                    reply!(false);
                } else {
                    let cmd: &str = SshCodec::decode(data)?;
                    let handler = self.handler.take().unwrap();
                    self.process = handler.on_exec_request(cmd);
                    reply!(self.process.is_some());
                }
            }
            "subsystem" => {
                if self.process.is_some() {
                    reply!(false);
                } else {
                    let subsystem: &str = SshCodec::decode(data)?;
                    let handler = self.handler.take().unwrap();
                    self.process = handler.on_subsystem_request(subsystem);
                    reply!(self.process.is_some());
                }
            }
            "signal" => {
                if let Some(proc) = self.process.as_mut() {
                    let signal: &str = SshCodec::decode(data)?;
                    reply!(proc.kill(signal).is_ok())
                } else {
                    reply!(false)
                }
            }
            _ => reply!(false),
        }
        Ok(())
    }

    fn poll(
        &mut self,
        cx: &mut Context,
        t: &mut Transport,
    ) -> Poll<Result<PollResult, ConnectionError>> {
        if !self.is_open {
            ready!(self.poll_open(cx, t))?;
        }
        if self.is_open {
            ready!(self.poll_window_adjust(cx, t))?;
            ready!(self.poll_replies(cx, t))?;
            ready!(self.poll_stderr_stdout(cx, t))?;
            ready!(self.poll_stdin(cx))?;
            ready!(self.poll_exit(cx, t))?;
            ready!(self.poll_close(cx, t))?;
        }
        ready!(t.poll_flush(cx))?;
        Poll::Ready(Ok(PollResult::Noop))
    }

    fn is_closed(&self) -> bool {
        self.is_close_rcvd && self.is_close_sent
    }
}

pub trait SessionHandler: std::fmt::Debug + Send + Sync + 'static {
    fn on_env_request(&mut self, key: String, val: String) -> Option<()>;
    fn on_pty_request(&mut self) -> Option<()>;
    fn on_shell_request(self: Box<Self>) -> Option<Box<dyn Process>>;
    fn on_exec_request(self: Box<Self>, cmd: &str) -> Option<Box<dyn Process>>;
    fn on_subsystem_request(self: Box<Self>, subsystem: &str) -> Option<Box<dyn Process>>;
}

#[derive(Debug)]
pub struct SessionRequest {
    tx: oneshot::Sender<Result<Box<dyn SessionHandler>, OpenFailure>>,
}

impl SessionRequest {
    pub fn accept(self, handler: Box<dyn SessionHandler>) {
        let _ = self.tx.send(Ok(handler));
    }
    pub fn reject(self, failure: OpenFailure) {
        let _ = self.tx.send(Err(failure));
    }
}

pub struct SessionHandle;
