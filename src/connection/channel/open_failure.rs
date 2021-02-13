#[derive(Copy, Clone, PartialEq, Eq)]
pub struct ChannelOpenFailure(pub u32);

impl ChannelOpenFailure {
    pub const ADMINISTRATIVELY_PROHIBITED: Self = Self(1);
    pub const OPEN_CONNECT_FAILED: Self = Self(2);
    pub const UNKNOWN_CHANNEL_TYPE: Self = Self(3);
    pub const RESOURCE_SHORTAGE: Self = Self(4);
}

impl std::fmt::Debug for ChannelOpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::ADMINISTRATIVELY_PROHIBITED => {
                write!(f, "ChannelOpenFailure::ADMINISTRATIVELY_PROHIBITED")
            }
            &Self::OPEN_CONNECT_FAILED => write!(f, "ChannelOpenFailure::OPEN_CONNECT_FAILED"),
            &Self::UNKNOWN_CHANNEL_TYPE => write!(f, "ChannelOpenFailure::UNKNOWN_CHANNEL_TYPE"),
            &Self::RESOURCE_SHORTAGE => write!(f, "ChannelOpenFailure::RESOURCE_SHORTAGE"),
            _ => write!(f, "ChannelOpenFailure({})", self.0),
        }
    }
}

impl std::error::Error for ChannelOpenFailure {}

impl std::fmt::Display for ChannelOpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}