use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgDisconnect<'a> {
    pub reason: DisconnectReason,
    pub description: &'a str,
    pub language: &'a str,
}

impl<'a> MsgDisconnect<'a> {
    pub fn new(reason: DisconnectReason) -> Self {
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
        let mut n = 1;
        n += self.reason.size();
        n += 4 + self.description.len();
        n += 4 + self.language.len();
        n
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push(&self.reason)?;
        e.push_str_framed(&self.description)?;
        e.push_str_framed(&self.language)
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
pub struct DisconnectReason(u32);

impl DisconnectReason {
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

impl Default for DisconnectReason {
    fn default() -> Self {
        Self::BY_APPLICATION
    }
}

impl Encode for DisconnectReason {
    fn size(&self) -> usize {
        4
    }
    fn encode<E: SshEncoder>(&self, c: &mut E) -> Option<()> {
        c.push_u32be(self.0)
    }
}

impl<'a> DecodeRef<'a> for DisconnectReason {
    fn decode<D: Decoder<'a>>(c: &mut D) -> Option<Self> {
        c.take_u32be().map(DisconnectReason)
    }
}

impl std::fmt::Debug for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::HOST_NOT_ALLOWED_TO_CONNECT => {
                write!(f, "DisconnectReason::HOST_NOT_ALLOWED_TO_CONNECT")
            }
            &Self::PROTOCOL_ERROR => write!(f, "DisconnectReason::PROTOCOL_ERROR"),
            &Self::KEY_EXCHANGE_FAILED => write!(f, "DisconnectReason::KEY_EXCHANGE_FAILED"),
            &Self::RESERVED => write!(f, "DisconnectReason::RESERVED"),
            &Self::MAC_ERROR => write!(f, "DisconnectReason::MAC_ERROR"),
            &Self::COMPRESSION_ERROR => write!(f, "DisconnectReason::COMPRESSION_ERROR"),
            &Self::SERVICE_NOT_AVAILABLE => write!(f, "DisconnectReason::SERVICE_NOT_AVAILABLE"),
            &Self::PROTOCOL_VERSION_NOT_SUPPORTED => {
                write!(f, "DisconnectReason::PROTOCOL_VERSION_NOT_SUPPORTED")
            }
            &Self::HOST_KEY_NOT_VERIFIABLE => {
                write!(f, "DisconnectReason::HOST_KEY_NOT_VERIFIABLE")
            }
            &Self::CONNECTION_LOST => write!(f, "DisconnectReason::CONNECTION_LOST"),
            &Self::BY_APPLICATION => write!(f, "DisconnectReason::BY_APPLICATION"),
            &Self::TOO_MANY_CONNECTIONS => write!(f, "DisconnectReason::TOO_MANY_CONNECTIONS"),
            &Self::AUTH_CANCELLED_BY_USER => write!(f, "DisconnectReason::AUTH_CANCELLED_BY_USER"),
            &Self::NO_MORE_AUTH_METHODS_AVAILABLE => {
                write!(f, "DisconnectReason::NO_MORE_AUTH_METHODS_AVAILABLE")
            }
            &Self::ILLEGAL_USER_NAME => write!(f, "DisconnectReason::ILLEGAL_USER_NAME"),
            _ => write!(f, "DisconnectReason({})", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgDisconnect {
            reason: DisconnectReason::MAC_ERROR,
            description: "description",
            language: "language",
        };
        assert_eq!("MsgDisconnect { reason: DisconnectReason::MAC_ERROR, description: \"description\", language: \"language\" }", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgDisconnect::new(DisconnectReason::MAC_ERROR);
        assert_eq!(
            &[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        assert_eq!(
            &Some(MsgDisconnect::new(DisconnectReason::MAC_ERROR)),
            &SliceDecoder::decode(&[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn test_reason_debug_01() {
        assert_eq!(
            "DisconnectReason::HOST_NOT_ALLOWED_TO_CONNECT",
            format!("{:?}", DisconnectReason::HOST_NOT_ALLOWED_TO_CONNECT)
        );
        assert_eq!(
            "DisconnectReason::PROTOCOL_ERROR",
            format!("{:?}", DisconnectReason::PROTOCOL_ERROR)
        );
        assert_eq!(
            "DisconnectReason::KEY_EXCHANGE_FAILED",
            format!("{:?}", DisconnectReason::KEY_EXCHANGE_FAILED)
        );
        assert_eq!(
            "DisconnectReason::RESERVED",
            format!("{:?}", DisconnectReason::RESERVED)
        );
        assert_eq!(
            "DisconnectReason::MAC_ERROR",
            format!("{:?}", DisconnectReason::MAC_ERROR)
        );
        assert_eq!(
            "DisconnectReason::COMPRESSION_ERROR",
            format!("{:?}", DisconnectReason::COMPRESSION_ERROR)
        );
        assert_eq!(
            "DisconnectReason::SERVICE_NOT_AVAILABLE",
            format!("{:?}", DisconnectReason::SERVICE_NOT_AVAILABLE)
        );
        assert_eq!(
            "DisconnectReason::PROTOCOL_VERSION_NOT_SUPPORTED",
            format!("{:?}", DisconnectReason::PROTOCOL_VERSION_NOT_SUPPORTED)
        );
        assert_eq!(
            "DisconnectReason::HOST_KEY_NOT_VERIFIABLE",
            format!("{:?}", DisconnectReason::HOST_KEY_NOT_VERIFIABLE)
        );
        assert_eq!(
            "DisconnectReason::CONNECTION_LOST",
            format!("{:?}", DisconnectReason::CONNECTION_LOST)
        );
        assert_eq!(
            "DisconnectReason::BY_APPLICATION",
            format!("{:?}", DisconnectReason::BY_APPLICATION)
        );
        assert_eq!(
            "DisconnectReason::TOO_MANY_CONNECTIONS",
            format!("{:?}", DisconnectReason::TOO_MANY_CONNECTIONS)
        );
        assert_eq!(
            "DisconnectReason::AUTH_CANCELLED_BY_USER",
            format!("{:?}", DisconnectReason::AUTH_CANCELLED_BY_USER)
        );
        assert_eq!(
            "DisconnectReason::NO_MORE_AUTH_METHODS_AVAILABLE",
            format!("{:?}", DisconnectReason::NO_MORE_AUTH_METHODS_AVAILABLE)
        );
        assert_eq!(
            "DisconnectReason::ILLEGAL_USER_NAME",
            format!("{:?}", DisconnectReason::ILLEGAL_USER_NAME)
        );
        assert_eq!(
            "DisconnectReason(16)",
            format!("{:?}", DisconnectReason(16))
        );
    }
}
