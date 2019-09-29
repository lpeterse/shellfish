use super::*;

pub enum RequestState<T> {
    None,
    Open(T),
    Progress,
    Success,
    Failure
}

impl <T> RequestState<T> {
    pub fn success(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Progress => return Ok(*self = Self::Success),
            _ => return Err(ConnectionError::ChannelSuccessUnexpected)
        }
    }
    pub fn failure(&mut self) -> Result<(), ConnectionError> {
        match self {
            Self::Progress => return Ok(*self = Self::Failure),
            _ => return Err(ConnectionError::ChannelFailureUnexpected)
        }
    }
}

#[derive(Debug)]
pub enum SessionRequest {
    EnvRequest(EnvRequest),
    PtyRequest(PtyRequest),
    ExecRequest(ExecRequest),
    ShellRequest(ShellRequest),
    SubsystemRequest(SubsystemRequest),
}

impl ChannelRequest for SessionRequest {
    fn name(&self) -> &'static str {
        match self {
            Self::EnvRequest(x) => x.name(),
            Self::PtyRequest(x) => x.name(),
            Self::ExecRequest(x) => x.name(),
            Self::ShellRequest(x) => x.name(),
            Self::SubsystemRequest(x) => x.name(),
        }
    }
}

impl Encode for SessionRequest {
    fn size(&self) -> usize {
        match self {
            Self::EnvRequest(x) => x.size(),
            Self::PtyRequest(x) => x.size(),
            Self::ExecRequest(x) => x.size(),
            Self::ShellRequest(x) => x.size(),
            Self::SubsystemRequest(x) => x.size(),
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::EnvRequest(x) => x.encode(e),
            Self::PtyRequest(x) => x.encode(e),
            Self::ExecRequest(x) => x.encode(e),
            Self::ShellRequest(x) => x.encode(e),
            Self::SubsystemRequest(x) => x.encode(e),
        }
    }
}

#[derive(Debug)]
pub struct EnvRequest {
    name: String,
    value: String,
}

impl ChannelRequest for EnvRequest {
    fn name(&self) -> &'static str {
        "env"
    }
}

impl Encode for EnvRequest {
    fn size(&self) -> usize {
        Encode::size(&self.name) +
        Encode::size(&self.value)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.name, e);
        Encode::encode(&self.value, e);
    }
}

#[derive(Debug)]
pub struct PtyRequest {

}

impl ChannelRequest for PtyRequest {
    fn name(&self) -> &'static str {
        "pty-req"
    }
}

impl Encode for PtyRequest {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, _e: &mut E) {
        // FIXME
    }
}

#[derive(Debug)]
pub struct ExecRequest {
    pub command: String
}

impl ChannelRequest for ExecRequest {
    fn name(&self) -> &'static str {
        "exec"
    }
}

impl Encode for ExecRequest {
    fn size(&self) -> usize {
        Encode::size(&self.command)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.command, e)
    }
}

#[derive(Debug)]
pub struct ShellRequest {

}

impl ChannelRequest for ShellRequest {
    fn name(&self) -> &'static str {
        "shell"
    }
}

impl Encode for ShellRequest {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, _e: &mut E) {
        // Nothing to do
    }
}

#[derive(Debug)]
pub struct SubsystemRequest {
    pub subsystem: String,
}

impl ChannelRequest for SubsystemRequest {
    fn name(&self) -> &'static str {
        "subsystem"
    }
}

impl Encode for SubsystemRequest {
    fn size(&self) -> usize {
        0
    }
    fn encode<E: Encoder>(&self, _e: &mut E) {
        // FIXME
    }
}

