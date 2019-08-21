use bytes::{BytesMut, BufMut};
use tokio::codec::{Encoder, Decoder};

use crate::parser::*;

#[derive(Copy, Clone, Debug)]
pub enum AgentRequest {
    RequestIdentities = 11,
}

pub enum AgentResponse {
    IdentitiesAnswer(Vec<(Vec<u8>,String)>),
    Unknown(u8)
}

pub struct AgentCodec {
    pub max_packet_size: usize
}

impl Default for AgentCodec {
    fn default() -> Self {
        Self { max_packet_size: 35000 }
    }
}

impl Encoder for AgentCodec {
    type Item = AgentRequest;
    type Error = AgentCodecError;

    fn encode(&mut self, req: AgentRequest, buf: &mut BytesMut) -> Result<(), AgentCodecError> {
        buf.reserve(5);
        buf.put_u8(0);
        buf.put_u8(0);
        buf.put_u8(0);
        buf.put_u8(1);
        match req {
            AgentRequest::RequestIdentities => buf.put_u8(req as u8),
        }
        Ok(())
    }
}

impl Decoder for AgentCodec {
    type Item = AgentResponse;
    type Error = AgentCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<AgentResponse>, AgentCodecError> {

        let mut p = Input::from(&mut buf[..]);

        let len: usize = match Parser::parse(&mut p) {
            None => return Ok(None), // not enough input
            Some(s) => s,
        };

        if len > self.max_packet_size {
            return Err(AgentCodecError::MaxPacketSizeExceeded(len));
        }

        if p.remaining() < len {
            return Ok(None); // not enough input
        }

        match Parser::parse(&mut p) {
            None => Err(AgentCodecError::SyntaxError),
            Some(r) => {
                buf.advance(4 + len); // remove from input buffer
                Ok(Some(r))
            }
        }
    }
}

#[derive(Debug)]
pub enum AgentCodecError {
    IoError(std::io::Error),
    MaxPacketSizeExceeded(usize),
    SyntaxError,
}

impl From<std::io::Error> for AgentCodecError {
    fn from(e: std::io::Error) -> Self {
        AgentCodecError::IoError(e)
    }
}

impl Parser for AgentResponse {
    fn parse(p: &mut Input) -> Option<Self> {
        match p.take_u8()? {
            12 => Parser::parse(p).map(AgentResponse::IdentitiesAnswer),
            n  => Some(AgentResponse::Unknown(n)),
        }
    }
}
