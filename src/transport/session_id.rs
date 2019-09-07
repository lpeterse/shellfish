pub enum SessionId {
    None,
    SessionId32([u8;32])
}

impl SessionId {
    pub fn is_uninitialized(&self) -> bool {
        match self {
            Self::None => true,
            _ => false,
        }
    }
}

impl From<[u8;32]> for SessionId {
    fn from(x: [u8;32]) -> Self {
        Self::SessionId32(x)
    }
}

impl AsRef<[u8]> for SessionId {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::None => &[],
            Self::SessionId32(x) => x.as_ref(),
        }
    }
}

impl std::fmt::Debug for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionId::None =>
                write!(f, "SessionId::None"),
            SessionId::SessionId32(id) =>
                write!(f, "SessionId ({:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x})",
                    id[00], id[01], id[02], id[03],
                    id[04], id[05], id[06], id[07],
                    id[08], id[09], id[10], id[11],
                    id[12], id[13], id[14], id[15],
                    id[16], id[17], id[18], id[19],
                    id[20], id[21], id[22], id[23],
                    id[24], id[25], id[26], id[27],
                    id[28], id[29], id[30], id[31])
        }
    }
}
