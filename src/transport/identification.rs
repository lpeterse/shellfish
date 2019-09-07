use crate::codec::*;

#[derive(Clone,Debug,PartialEq)]
pub struct Identification {
    pub version: String,
    pub comment: Option<String>,
}

impl Identification {
    pub const MAX_LEN: usize = 253;

    pub fn new(version: String) -> Self {
        Self {
            version,
            comment: None,
        }
    }
}

impl Default for Identification {
    fn default() -> Self {
        Self {
            version: format!("{}_{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            comment: None
        }
    }
}

impl <'a> Codec<'a> for Identification {
    fn size(&self) -> usize {
        b"SSH-2.0-".len()
        + self.version.len()
        + match self.comment { None => 0, Some(ref x) => 1 + x.len() }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_bytes(b"SSH-2.0-");
        e.push_bytes(&self.version.as_bytes());
        match self.comment { None => (), Some(ref x) => { e.push_u8(' ' as u8); e.push_bytes(&x.as_bytes()); }};
    }
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_match(b"SSH-2.0-")?;
        if d.remaining() > Self::MAX_LEN { return None };
        let version = d.take_while(|x| (x as char).is_ascii_graphic() && x != ('-' as u8) && x != (' ' as u8))?;
        if d.is_eoi() {
            Some(Self { version: String::from_utf8(version.to_vec()).ok()?, comment: None })
        } else {
            d.take_u8().filter(|x| *x == (' ' as u8))?;
            let comment = d.take_while(|x| (x as char).is_ascii_graphic())?;
            d.take_eoi()?;
            Some(Self { version: String::from_utf8(version.to_vec()).ok()?, comment: Some(String::from_utf8(comment.to_vec()).ok()?) })
        }
    }
}
