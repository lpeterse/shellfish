#[derive(Copy, Clone, PartialEq, Eq)]
pub struct OpenFailure(pub u32);

impl OpenFailure {
    pub const ADMINISTRATIVELY_PROHIBITED: Self = Self(1);
    pub const OPEN_CONNECT_FAILED: Self = Self(2);
    pub const UNKNOWN_CHANNEL_TYPE: Self = Self(3);
    pub const RESOURCE_SHORTAGE: Self = Self(4);
}

impl std::fmt::Debug for OpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &Self::ADMINISTRATIVELY_PROHIBITED => {
                write!(f, "OpenFailure::ADMINISTRATIVELY_PROHIBITED")
            }
            &Self::OPEN_CONNECT_FAILED => write!(f, "OpenFailure::OPEN_CONNECT_FAILED"),
            &Self::UNKNOWN_CHANNEL_TYPE => write!(f, "OpenFailure::UNKNOWN_CHANNEL_TYPE"),
            &Self::RESOURCE_SHORTAGE => write!(f, "OpenFailure::RESOURCE_SHORTAGE"),
            _ => write!(f, "OpenFailure({})", self.0),
        }
    }
}

impl std::error::Error for OpenFailure {}

impl std::fmt::Display for OpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
