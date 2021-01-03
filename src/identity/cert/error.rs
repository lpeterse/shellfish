#[derive(Debug, Clone, Copy)]
pub enum CertError {
    InvalidType,
    InvalidSignature,
    InvalidPrincipal,
    InvalidPeriod,
    InvalidSource,
    InvalidOptions,
}

impl std::fmt::Display for CertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidType => write!(f, "Invalid type (host/user)"),
            Self::InvalidSignature => write!(f, "Invalid signature"),
            Self::InvalidPrincipal => write!(f, "Invalid principal"),
            Self::InvalidPeriod => write!(f, "Invalid validity period"),
            Self::InvalidSource => write!(f, "Invalid source address"),
            Self::InvalidOptions => write!(f, "Invalid critical options"),
        }
    }
}

impl std::error::Error for CertError {}
