use super::super::super::{ConnectionError, ConnectionErrorWatch};
use super::super::state::ChannelState;
use super::super::OpenFailure;
use super::Process;
use super::{super::super::msg::*, PtySpecification};
use super::{Session, SessionReq, SessionRun};
use crate::ready;
use crate::transport::GenericTransport;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct SessionClientState0 {
    lid: u32,
    lws: u32,
    lps: u32,
    res: oneshot::Sender<Result<Session, OpenFailure>>,
    err: ConnectionErrorWatch,
}

impl SessionClientState0 {
    pub fn new(
        lid: u32,
        lws: u32,
        lps: u32,
        res: oneshot::Sender<Result<Session, OpenFailure>>,
        err: ConnectionErrorWatch,
    ) -> Self {
        Self {
            lid,
            lws,
            lps,
            res,
            err,
        }
    }
}

impl ChannelState for SessionClientState0 {
    fn on_open_confirmation(
        self: Box<Self>,
        rid: u32,
        rws: u32,
        rps: u32,
    ) -> Result<Box<dyn ChannelState>, ConnectionError> {
        let (req_tx, req_nxt) = oneshot::channel();
        let mut st = SessionClientState1 {
            lid: self.lid,
            lws: self.lws,
            lps: self.lps,
            rid,
            rws,
            rps,
            close: false,
            close_sent: false,
            close_rcvd: false,
            req_env: None,
            req_pty: None,
            req_run: None,
            req_nxt: Some(req_nxt),
        };
        let ss = Session {
            req_tx,
            error: self.err.clone(),
        };
        st.close = self.res.send(Ok(ss)).is_err();
        Ok(Box::new(st))
    }

    fn on_open_failure(self: Box<Self>, e: OpenFailure) -> Result<(), ConnectionError> {
        let _ = self.res.send(Err(e));
        Ok(())
    }

    fn poll_with_transport(
        &mut self,
        _cx: &mut Context,
        _t: &mut GenericTransport,
    ) -> Poll<Result<bool, ConnectionError>> {
        Poll::Ready(Ok(false))
    }
}

#[derive(Debug)]
pub struct SessionClientState1 {
    lid: u32,
    lws: u32,
    lps: u32,
    rid: u32,
    rws: u32,
    rps: u32,
    close: bool,
    close_sent: bool,
    close_rcvd: bool,
    req_env: Option<(String, String)>,
    req_pty: Option<(Option<PtySpecification>, oneshot::Sender<bool>)>,
    req_run: Option<(Option<SessionRun>, oneshot::Sender<Process>)>,
    req_nxt: Option<oneshot::Receiver<SessionReq>>,
}

impl ChannelState for SessionClientState1 {
    fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut GenericTransport,
    ) -> Poll<Result<bool, ConnectionError>> {
        loop {
            if self.close {
                if !self.close_sent {
                    let msg = MsgChannelClose::new(self.rid);
                    ready!(t.poll_send(cx, &msg))?;
                    self.close_sent = true;
                }
                return Poll::Ready(Ok(self.close_rcvd));
            }
            if let Some(specific) = &self.req_env {
                let msg = MsgChannelRequest {
                    recipient_channel: self.rid,
                    request: "env",
                    want_reply: false,
                    specific: (specific.0.as_str(), specific.1.as_str()),
                };
                ready!(t.poll_send(cx, &msg))?;
                self.req_env = None;
            }
            if let Some(ref mut req) = &mut self.req_pty {
                if let Some(pty) = &req.0 {
                    let msg = MsgChannelRequest {
                        recipient_channel: self.rid,
                        request: "pty-req",
                        want_reply: false,
                        specific: (), // FIXME
                    };
                    ready!(t.poll_send(cx, &msg))?;
                    req.0 = None;
                }
                break;
            }
            if let Some(ref mut req) = &mut self.req_run {
                if let Some(run) = &req.0 {
                    match run {
                        SessionRun::Shell => {
                            let msg = MsgChannelRequest {
                                recipient_channel: self.rid,
                                request: "shell",
                                want_reply: true,
                                specific: (),
                            };
                            ready!(t.poll_send(cx, &msg))?;
                        }
                        SessionRun::Exec(command) => {
                            let msg = MsgChannelRequest {
                                recipient_channel: self.rid,
                                request: "exec",
                                want_reply: true,
                                specific: command.as_str(),
                            };
                            ready!(t.poll_send(cx, &msg))?;
                        }
                        SessionRun::Subsystem(subsystem) => {
                            let msg = MsgChannelRequest {
                                recipient_channel: self.rid,
                                request: "subsystem",
                                want_reply: true,
                                specific: subsystem.as_str(),
                            };
                            ready!(t.poll_send(cx, &msg))?;
                        }
                    }
                    req.0 = None;
                }
                break;
            }
            // Poll whether the session handle sent a request.
            // Correctness by design: No request can follow after shell/exec/subsystem.
            if let Some(ref mut req_nxt) = &mut self.req_nxt {
                match Future::poll(Pin::new(req_nxt), cx) {
                    Poll::Ready(Err(_)) => {
                        self.close = true;
                        continue;
                    }
                    Poll::Ready(Ok(SessionReq::Env { env, res, nxt })) => {
                        self.req_nxt = Some(nxt);
                        self.req_env = Some(env);
                        self.close = res.send(()).is_err();
                        continue;
                    }
                    Poll::Ready(Ok(SessionReq::Pty { pty, res, nxt })) => {
                        self.req_nxt = Some(nxt);
                        self.req_pty = Some((Some(pty), res));
                        continue;
                    }
                    Poll::Ready(Ok(SessionReq::Run { run, res })) => {
                        self.req_nxt = None;
                        self.req_run = Some((Some(run), res));
                        continue;
                    }
                    Poll::Pending => (),
                }
            }
            // Break the loop as there's nothing to do (no request atm).
            break;
        }
        Poll::Ready(Ok(false))
    }
}
