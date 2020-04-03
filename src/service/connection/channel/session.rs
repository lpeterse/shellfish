mod channel;
mod exit;
mod process;
mod request;
mod signal;

pub(crate) use self::channel::*;
pub(crate) use self::exit::*;
pub(crate) use self::process::*;
pub(crate) use self::request::*;
pub(crate) use self::signal::*;

use super::super::*;
use super::*;

use crate::buffer::*;
use crate::codec::*;

/// A session is a remote execution of a program.  The program may be a
/// shell, an application, a system command, or some built-in subsystem.
/// It may or may not have a tty, and may or may not involve X11
/// forwarding.  Multiple sessions can be active simultaneously.
pub struct Session(pub(crate) SessionState);

impl Session {
    /// Execute a remote shell.
    pub async fn shell(self) -> Result<Process, ConnectionError> {
        let req = SessionRequest::ShellRequest(ShellRequest {});
        Ok(Process::new(self.request(req).await?))
    }

    /// Execute a command.
    pub async fn exec(self, command: String) -> Result<Process, ConnectionError> {
        let req = SessionRequest::ExecRequest(ExecRequest { command });
        Ok(Process::new(self.request(req).await?))
    }

    /// Execute a subsystem.
    pub async fn subsystem(self, subsystem: String) -> Result<Process, ConnectionError> {
        let req = SessionRequest::SubsystemRequest(SubsystemRequest { subsystem });
        Ok(Process::new(self.request(req).await?))
    }

    async fn request(self, request: SessionRequest) -> Result<Self, ConnectionError> {
        let mut state = (self.0).0.lock().map_err(|_| ConnectionError::Terminated)?;
        state.request = RequestState::Open(request);
        //state.inner_wake(); // FIXME
        drop(state);
        async_std::future::poll_fn(|cx| {
            let mut state = (self.0).0.lock().map_err(|_| ConnectionError::Terminated)?;
            match state.request {
                RequestState::Success => {
                    state.request = RequestState::None;
                    Poll::Ready(Ok(()))
                }
                RequestState::Failure => {
                    state.request = RequestState::None;
                    Poll::Ready(Err(ConnectionError::ChannelRequestFailure))
                }
                _ => {
                    // state.outer_register(cx); // FIXME
                    Poll::Pending
                }
            }
        })
        .await?;
        Ok(self)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        match (self.0).0.lock() {
            Err(_) => (),
            Ok(mut state) => {
                state.outer_done = Some(());
                //state.inner_wake(); // FIXME
            }
        }
    }
}

impl Channel for Session {
    type Open = ();
    type Confirmation = ();
    type Request = SessionRequest;
    type State = SessionState;

    const NAME: &'static str = "session";

    fn new_state(max_buffer_size: usize) -> Self::State {
        todo!()
    }
}


        /*
        match channel.shared() {
            SharedState::Session(ref st) => {
                let mut state = st.lock().unwrap();
                match msg.request {
                    "env" => {
                        let env = BDecoder::decode(msg.specific)
                            .ok_or(TransportError::DecoderError)?;
                        //state.add_env(env); FIXME
                    }
                    "exit-status" => {
                        let status = BDecoder::decode(msg.specific)
                            .ok_or(TransportError::DecoderError)?;
                        state.set_exit_status(status);
                    }
                    "exit-signal" => {
                        let signal = BDecoder::decode(msg.specific)
                            .ok_or(TransportError::DecoderError)?;
                        state.set_exit_signal(signal);
                    }
                    _ => {
                        if msg.want_reply {
                            let msg = MsgChannelFailure {
                                recipient_channel: channel.remote_channel(),
                            };
                            ready!(x.transport.poll_send(cx, &msg))?;
                            log::debug!("Sent MSG_CHANNEL_FAILURE");
                        }
                    }
                }
            }
        }*/