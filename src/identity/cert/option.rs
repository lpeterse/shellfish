use crate::util::codec::*;

const FORCE_COMMAND: &'static str = "force-command";
const SOURCE_ADDRESS: &'static str = "source-address";

#[derive(Clone, Debug, PartialEq)]
pub enum CertOption {
    ForceCommand(String),
    SourceAddress(String),
    Other(String, Vec<u8>),
}

impl SshEncode for CertOption {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        match self {
            Self::ForceCommand(cmd) => {
                e.push_str_framed(FORCE_COMMAND)?;
                e.push_usize(SshCodec::size(cmd)?)?;
                e.push_str_framed(cmd)?;
            }
            Self::SourceAddress(addr) => {
                e.push_str_framed(SOURCE_ADDRESS)?;
                e.push_usize(SshCodec::size(addr)?)?;
                e.push_str_framed(addr)?;
            }
            Self::Other(name, data) => {
                e.push_str_framed(name)?;
                e.push_bytes_framed(data)?;
            }
        }
        Some(())
    }
}

impl SshDecode for CertOption {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        let name = d.take_str_framed()?;
        let data = d.take_bytes_framed()?;
        Some(match &name {
            &FORCE_COMMAND => Self::ForceCommand(SshCodec::decode(data)?),
            &SOURCE_ADDRESS => Self::SourceAddress(SshCodec::decode(data)?),
            _ => Self::Other(name.into(), data.into()),
        })
    }
}
