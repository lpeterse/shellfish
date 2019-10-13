use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgDisconnect<'a> {
    pub reason: Reason,
    pub description: &'a str,
    pub language: &'a str,
}

impl<'a> MsgDisconnect<'a> {
    pub fn new(reason: Reason) -> Self {
        Self {
            reason,
            description: "",
            language: "",
        }
    }
}

impl<'a> Message for MsgDisconnect<'a> {
    const NUMBER: u8 = 1;
}

impl<'a> Encode for MsgDisconnect<'a> {
    fn size(&self) -> usize {
        1 + Encode::size(&self.reason)
            + Encode::size(&self.description)
            + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u8(<Self as Message>::NUMBER);
        Encode::encode(&self.reason, c);
        Encode::encode(&self.description, c);
        Encode::encode(&self.language, c);
    }
}

impl<'a> DecodeRef<'a> for MsgDisconnect<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            reason: DecodeRef::decode(d)?,
            description: DecodeRef::decode(d)?,
            language: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Reason(u32);

impl Reason {
    pub const HOST_NOT_ALLOWED_TO_CONNECT: Self = Self(1);
    pub const PROTOCOL_ERROR: Self = Self(2);
    pub const KEY_EXCHANGE_FAILED: Self = Self(3);
    pub const RESERVED: Self = Self(4);
    pub const MAC_ERROR: Self = Self(5);
    pub const COMPRESSION_ERROR: Self = Self(6);
    pub const SERVICE_NOT_AVAILABLE: Self = Self(7);
    pub const PROTOCOL_VERSION_NOT_SUPPORTED: Self = Self(8);
    pub const HOST_KEY_NOT_VERIFIABLE: Self = Self(9);
    pub const CONNECTION_LOST: Self = Self(10);
    pub const BY_APPLICATION: Self = Self(11);
    pub const TOO_MANY_CONNECTIONS: Self = Self(12);
    pub const AUTH_CANCELLED_BY_USER: Self = Self(13);
    pub const NO_MORE_AUTH_METHODS_AVAILABLE: Self = Self(14);
    pub const ILLEGAL_USER_NAME: Self = Self(15);
}

impl Encode for Reason {
    fn size(&self) -> usize {
        4
    }
    fn encode<E: Encoder>(&self, c: &mut E) {
        c.push_u32be(self.0)
    }
}

impl<'a> DecodeRef<'a> for Reason {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().map(Reason)
    }
}

impl std::fmt::Debug for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::HOST_NOT_ALLOWED_TO_CONNECT => write!(f, "Reason::HOST_NOT_ALLOWED_TO_CONNECT"),
            &Self::PROTOCOL_ERROR => write!(f, "Reason::PROTOCOL_ERROR"),
            &Self::KEY_EXCHANGE_FAILED => write!(f, "Reason::KEY_EXCHANGE_FAILED"),
            &Self::RESERVED => write!(f, "Reason::RESERVED"),
            &Self::MAC_ERROR => write!(f, "Reason::MAC_ERROR"),
            &Self::COMPRESSION_ERROR => write!(f, "Reason::COMPRESSION_ERROR"),
            &Self::SERVICE_NOT_AVAILABLE => write!(f, "Reason::SERVICE_NOT_AVAILABLE"),
            &Self::PROTOCOL_VERSION_NOT_SUPPORTED => {
                write!(f, "Reason::PROTOCOL_VERSION_NOT_SUPPORTED")
            }
            &Self::HOST_KEY_NOT_VERIFIABLE => write!(f, "Reason::HOST_KEY_NOT_VERIFIABLE"),
            &Self::CONNECTION_LOST => write!(f, "Reason::CONNECTION_LOST"),
            &Self::BY_APPLICATION => write!(f, "Reason::BY_APPLICATION"),
            &Self::TOO_MANY_CONNECTIONS => write!(f, "Reason::TOO_MANY_CONNECTIONS"),
            &Self::AUTH_CANCELLED_BY_USER => write!(f, "Reason::AUTH_CANCELLED_BY_USER"),
            &Self::NO_MORE_AUTH_METHODS_AVAILABLE => {
                write!(f, "Reason::NO_MORE_AUTH_METHODS_AVAILABLE")
            }
            &Self::ILLEGAL_USER_NAME => write!(f, "Reason::ILLEGAL_USER_NAME"),
            _ => write!(f, "Reason({})", self.0),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgDisconnect {
            reason: Reason::MAC_ERROR,
            description: "description",
            language: "language",
        };
        assert_eq!("MsgDisconnect { reason: Reason::MAC_ERROR, description: \"description\", language: \"language\" }", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgDisconnect::new(Reason::MAC_ERROR);
        assert_eq!(
            &[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        assert_eq!(
            &Some(MsgDisconnect::new(Reason::MAC_ERROR)),
            &BDecoder::decode(&[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn test_reason_debug_01() {
        assert_eq!(
            "Reason::HOST_NOT_ALLOWED_TO_CONNECT",
            format!("{:?}", Reason::HOST_NOT_ALLOWED_TO_CONNECT)
        );
        assert_eq!(
            "Reason::PROTOCOL_ERROR",
            format!("{:?}", Reason::PROTOCOL_ERROR)
        );
        assert_eq!(
            "Reason::KEY_EXCHANGE_FAILED",
            format!("{:?}", Reason::KEY_EXCHANGE_FAILED)
        );
        assert_eq!("Reason::RESERVED", format!("{:?}", Reason::RESERVED));
        assert_eq!("Reason::MAC_ERROR", format!("{:?}", Reason::MAC_ERROR));
        assert_eq!(
            "Reason::COMPRESSION_ERROR",
            format!("{:?}", Reason::COMPRESSION_ERROR)
        );
        assert_eq!(
            "Reason::SERVICE_NOT_AVAILABLE",
            format!("{:?}", Reason::SERVICE_NOT_AVAILABLE)
        );
        assert_eq!(
            "Reason::PROTOCOL_VERSION_NOT_SUPPORTED",
            format!("{:?}", Reason::PROTOCOL_VERSION_NOT_SUPPORTED)
        );
        assert_eq!(
            "Reason::HOST_KEY_NOT_VERIFIABLE",
            format!("{:?}", Reason::HOST_KEY_NOT_VERIFIABLE)
        );
        assert_eq!(
            "Reason::CONNECTION_LOST",
            format!("{:?}", Reason::CONNECTION_LOST)
        );
        assert_eq!(
            "Reason::BY_APPLICATION",
            format!("{:?}", Reason::BY_APPLICATION)
        );
        assert_eq!(
            "Reason::TOO_MANY_CONNECTIONS",
            format!("{:?}", Reason::TOO_MANY_CONNECTIONS)
        );
        assert_eq!(
            "Reason::AUTH_CANCELLED_BY_USER",
            format!("{:?}", Reason::AUTH_CANCELLED_BY_USER)
        );
        assert_eq!(
            "Reason::NO_MORE_AUTH_METHODS_AVAILABLE",
            format!("{:?}", Reason::NO_MORE_AUTH_METHODS_AVAILABLE)
        );
        assert_eq!(
            "Reason::ILLEGAL_USER_NAME",
            format!("{:?}", Reason::ILLEGAL_USER_NAME)
        );
        assert_eq!("Reason(16)", format!("{:?}", Reason(16)));
    }
}
