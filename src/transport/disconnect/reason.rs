use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Reason {
    HostNotAllowedToConnect,
    ProtocolError,
    KeyExchangeFailed,
    Reserved,
    MacError,
    CompressionError,
    ServiceNotAvailable,
    ProtocolVersionNotSupported,
    HostKeyNotVerifiable,
    ConnectionLost,
    ByApplication,
    TooManyConnections,
    AuthCancelledByUser,
    NoMoreAuthMethodsAvailable,
    IllegalUserName,
    Other(u32),
}

impl<'a> Codec<'a> for Reason {
    fn size(&self) -> usize {
        4
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(match self {
            Self::HostNotAllowedToConnect => 1,
            Self::ProtocolError => 2,
            Self::KeyExchangeFailed => 3,
            Self::Reserved => 4,
            Self::MacError => 5,
            Self::CompressionError => 6,
            Self::ServiceNotAvailable => 7,
            Self::ProtocolVersionNotSupported => 8,
            Self::HostKeyNotVerifiable => 9,
            Self::ConnectionLost => 10,
            Self::ByApplication => 11,
            Self::TooManyConnections => 12,
            Self::AuthCancelledByUser => 13,
            Self::NoMoreAuthMethodsAvailable => 14,
            Self::IllegalUserName => 15,
            Self::Other(reason) => *reason,
        })
    }
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().map(|x| match x {
            1 => Self::HostNotAllowedToConnect,
            2 => Self::ProtocolError,
            3 => Self::KeyExchangeFailed,
            4 => Self::Reserved,
            5 => Self::MacError,
            6 => Self::CompressionError,
            7 => Self::ServiceNotAvailable,
            8 => Self::ProtocolVersionNotSupported,
            9 => Self::HostKeyNotVerifiable,
            10 => Self::ConnectionLost,
            11 => Self::ByApplication,
            12 => Self::TooManyConnections,
            13 => Self::AuthCancelledByUser,
            14 => Self::NoMoreAuthMethodsAvailable,
            15 => Self::IllegalUserName,
            r => Self::Other(r),
        })
    }
}
