use crate::codec::*;

pub enum Exit {
    Status(ExitStatus),
    Signal(ExitSignal),
}

pub struct ExitStatus(pub u32);

#[derive(Debug, Clone)]
pub struct ExitSignal {
    signal: Signal,
    core_dumped: bool,
    message: String,
}

#[derive(Debug, Clone)]
pub enum Signal {
    ABRT,
    ALRM,
    FPE,
    HUP,
    ILL,
    INT,
    KILL,
    PIPE,
    QUIT,
    SEGV,
    TERM,
    USR1,
    USR2,
    Other(String),
}

impl Encode for ExitStatus {
    fn size(&self) -> usize {
        std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(self.0)
    }
}

impl Decode for ExitStatus {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self(d.take_u32be()?).into()
    }
}

impl Encode for ExitSignal {
    fn size(&self) -> usize {
        Encode::size(&self.signal) + 1 + Encode::size(&self.message) + Encode::size(&"")
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.signal, e);
        e.push_u8(self.core_dumped as u8);
        Encode::encode(&self.message, e);
        Encode::encode(&"", e);
    }
}

impl Decode for ExitSignal {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let signal = Decode::decode(d)?;
        let core_dumped = d.take_bool()?;
        let message = Decode::decode(d)?;
        let _: &str = DecodeRef::decode(d)?;
        Self {
            signal,
            core_dumped,
            message,
        }
        .into()
    }
}

impl Encode for Signal {
    fn size(&self) -> usize {
        match self {
            Self::ABRT => Encode::size(&"ABRT"),
            Self::ALRM => Encode::size(&"ALRM"),
            Self::FPE => Encode::size(&"FPE"),
            Self::HUP => Encode::size(&"HUP"),
            Self::ILL => Encode::size(&"ILL"),
            Self::INT => Encode::size(&"INT"),
            Self::KILL => Encode::size(&"KILL"),
            Self::PIPE => Encode::size(&"PIPE"),
            Self::QUIT => Encode::size(&"QUIT"),
            Self::SEGV => Encode::size(&"SEGV"),
            Self::TERM => Encode::size(&"TERM"),
            Self::USR1 => Encode::size(&"USR1"),
            Self::USR2 => Encode::size(&"USR2"),
            Self::Other(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::ABRT => Encode::encode(&"ABRT", e),
            Self::ALRM => Encode::encode(&"ALRM", e),
            Self::FPE => Encode::encode(&"FPE", e),
            Self::HUP => Encode::encode(&"HUP", e),
            Self::ILL => Encode::encode(&"ILL", e),
            Self::INT => Encode::encode(&"INT", e),
            Self::KILL => Encode::encode(&"KILL", e),
            Self::PIPE => Encode::encode(&"PIPE", e),
            Self::QUIT => Encode::encode(&"QUIT", e),
            Self::SEGV => Encode::encode(&"SEGV", e),
            Self::TERM => Encode::encode(&"TERM", e),
            Self::USR1 => Encode::encode(&"USR1", e),
            Self::USR2 => Encode::encode(&"USR2", e),
            Self::Other(x) => Encode::encode(x, e),
        }
    }
}

impl Decode for Signal {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        match DecodeRef::decode(d)? {
            "ABRT" => Self::ABRT,
            "ALRM" => Self::ALRM,
            "FPE" => Self::FPE,
            "HUP" => Self::HUP,
            "ILL" => Self::ILL,
            "INT" => Self::INT,
            "KILL" => Self::KILL,
            "PIPE" => Self::PIPE,
            "QUIT" => Self::QUIT,
            "SEGV" => Self::SEGV,
            "TERM" => Self::TERM,
            "USR1" => Self::USR1,
            "USR2" => Self::USR2,
            x => Self::Other(x.into()),
        }
        .into()
    }
}
