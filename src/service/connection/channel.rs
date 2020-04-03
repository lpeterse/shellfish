mod direct_tcpip;
mod session;

use super::*;

pub use self::direct_tcpip::*;
pub use self::session::*;

pub(crate) trait Channel: Sized {
    type Open: Clone + Encode + Decode;
    type Confirmation: Encode + Decode;
    type Request: ChannelRequest + Encode;
    type State: ChannelState;

    const NAME: &'static str;

    fn new_state(max_buffer_size: usize) -> Self::State;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

impl ChannelRequest for () {
    fn name(&self) -> &'static str {
        ""
    }
}

pub(crate) trait ChannelState: Send {
    fn terminate(&mut self, e: ConnectionError);

    fn push_open_confirmation(&mut self, id: u32, ws: u32, ps: u32) -> Result<(), ConnectionError>;
    fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError>;
    fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError>;
    fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError>;
    fn push_eof(&mut self) -> Result<(), ConnectionError>;
    fn push_close(&mut self) -> Result<(), ConnectionError>;
    fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError>;

    fn push_request(&mut self, request: &[u8]) -> Result<(), ConnectionError>;
    fn push_success(&mut self) -> Result<(), ConnectionError>;
    fn push_failure(&mut self) -> Result<(), ConnectionError>;

    fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>>;
}

pub(crate) enum ChannelState2 {
    Session(SessionState),
    DirectTcpIp(DirectTcpIpState),
}

impl ChannelState for ChannelState2 {
    fn push_open_confirmation(&mut self, ch: u32, ws: u32, ps: u32) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_open_confirmation(ch, ws, ps),
            Self::DirectTcpIp(x) => x.push_open_confirmation(ch, ws, ps),
        }
    }
    fn push_open_failure(
        &mut self,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_open_failure(reason),
            Self::DirectTcpIp(x) => x.push_open_failure(reason),
        }
    }
    fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_data(data),
            Self::DirectTcpIp(x) => x.push_data(data),
        }
    }
    fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_extended_data(code, data),
            Self::DirectTcpIp(x) => x.push_extended_data(code, data),
        }
    }
    fn push_eof(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_eof(),
            Self::DirectTcpIp(x) => x.push_eof(),
        }
    }
    fn push_close(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_close(),
            Self::DirectTcpIp(x) => x.push_close(),
        }
    }
    fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_window_adjust(n),
            Self::DirectTcpIp(x) => x.push_window_adjust(n),
        }
    }
    fn push_request(&mut self, request: &[u8]) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_request(request),
            Self::DirectTcpIp(x) => x.push_request(request),
        }
    }
    fn push_success(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_success(),
            Self::DirectTcpIp(x) => x.push_success(),
        }
    }
    fn push_failure(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Session(x) => x.push_failure(),
            Self::DirectTcpIp(x) => x.push_failure(),
        }
    }
    fn poll<T: TransportLayer>(
        &self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        match self {
            Self::Session(x) => x.poll(cx, t),
            Self::DirectTcpIp(x) => x.poll(cx, t),
        }
    }
    fn terminate(&mut self, e: ConnectionError) {
        match self {
            Self::Session(x) => x.terminate(e),
            Self::DirectTcpIp(x) => x.terminate(e),
        }
    }
}

impl From<SessionState> for ChannelState2 {
    fn from(x: SessionState) -> Self {
        Self::Session(x)
    }
}

impl From<DirectTcpIpState> for ChannelState2 {
    fn from(x: DirectTcpIpState) -> Self {
        Self::DirectTcpIp(x)
    }
}
