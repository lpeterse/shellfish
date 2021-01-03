#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DisconnectReason(pub u32);

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

impl std::fmt::Debug for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Display for DisconnectReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::HOST_NOT_ALLOWED_TO_CONNECT => write!(f, "HOST_NOT_ALLOWED_TO_CONNECT"),
            &Self::PROTOCOL_ERROR => write!(f, "PROTOCOL_ERROR"),
            &Self::KEY_EXCHANGE_FAILED => write!(f, "KEY_EXCHANGE_FAILED"),
            &Self::RESERVED => write!(f, "RESERVED"),
            &Self::MAC_ERROR => write!(f, "MAC_ERROR"),
            &Self::COMPRESSION_ERROR => write!(f, "COMPRESSION_ERROR"),
            &Self::SERVICE_NOT_AVAILABLE => write!(f, "SERVICE_NOT_AVAILABLE"),
            &Self::PROTOCOL_VERSION_NOT_SUPPORTED => write!(f, "PROTOCOL_VERSION_NOT_SUPPORTED"),
            &Self::HOST_KEY_NOT_VERIFIABLE => write!(f, "HOST_KEY_NOT_VERIFIABLE"),
            &Self::CONNECTION_LOST => write!(f, "CONNECTION_LOST"),
            &Self::BY_APPLICATION => write!(f, "BY_APPLICATION"),
            &Self::TOO_MANY_CONNECTIONS => write!(f, "TOO_MANY_CONNECTIONS"),
            &Self::AUTH_CANCELLED_BY_USER => write!(f, "AUTH_CANCELLED_BY_USER"),
            &Self::NO_MORE_AUTH_METHODS_AVAILABLE => write!(f, "NO_MORE_AUTH_METHODS_AVAILABLE"),
            &Self::ILLEGAL_USER_NAME => write!(f, "ILLEGAL_USER_NAME"),
            &Self(n) => write!(f, "{}", n),
        }
    }
}
