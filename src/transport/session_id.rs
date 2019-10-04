use crate::codec::*;

#[derive(Copy, Clone)]
pub enum SessionId {
    None,
    SessionId32([u8;32])
}

impl SessionId {
    pub fn new() -> Self {
        Self::None
    }
    pub fn set_if_uninitialized(&mut self, x: [u8;32]) {
        // The session id must only be set once, just ignore new values.
        match self {
            Self::None => *self = SessionId::SessionId32(x),
            _ => (),
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
            Self::None => panic!("session id uninitialized"),
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

impl Encode for SessionId {
    fn size(&self) -> usize {
        4 + match self {
            Self::None => 0,
            Self::SessionId32(_) => 32,
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::None => {},
            Self::SessionId32(id) => {
                e.push_u32be(32);
                e.push_bytes(id);
            }
        }
    }
}

impl<'a> DecodeRef<'a> for SessionId {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        match d.take_u32be()? {
            32 => {
                let mut x = [0;32];
                d.take_into(&mut x);
                Some(Self::SessionId32(x))
            }
            _ => None,
        }
    }
}
