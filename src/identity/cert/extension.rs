use crate::util::codec::*;

const NO_PRESENCE_REQUIRED: &'static str = "no-presence-required";
const PERMIT_X11_FORWARDING: &'static str = "permit-X11-forwarding";
const PERMIT_AGENT_FORWARDING: &'static str = "permit-agent-forwarding";
const PERMIT_PORT_FORWARDING: &'static str = "permit-port-forwarding";
const PERMIT_PTY: &'static str = "permit-pty";
const PERMIT_USER_RC: &'static str = "permit-user-rc";

#[derive(Clone, Debug, PartialEq)]
pub enum CertExtension {
    NoPresenceRequired,
    PermitX11Forwarding,
    PermitAgentForwarding,
    PermitPortForwarding,
    PermitPty,
    PermitUserRc,
    Other(String, Vec<u8>),
}

impl SshEncode for CertExtension {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        match self {
            Self::NoPresenceRequired => {
                e.push_str_framed(NO_PRESENCE_REQUIRED)?;
                e.push_bytes_framed(b"")?;
            }
            Self::PermitX11Forwarding => {
                e.push_str_framed(PERMIT_X11_FORWARDING)?;
                e.push_bytes_framed(b"")?;
            }
            Self::PermitAgentForwarding => {
                e.push_str_framed(PERMIT_AGENT_FORWARDING)?;
                e.push_bytes_framed(b"")?;
            }
            Self::PermitPortForwarding => {
                e.push_str_framed(PERMIT_PORT_FORWARDING)?;
                e.push_bytes_framed(b"")?;
            }
            Self::PermitPty => {
                e.push_str_framed(PERMIT_PTY)?;
                e.push_bytes_framed(b"")?;
            }
            Self::PermitUserRc => {
                e.push_str_framed(PERMIT_USER_RC)?;
                e.push_bytes_framed(b"")?;
            }
            Self::Other(name, data) => {
                e.push_str_framed(name)?;
                e.push_bytes_framed(data)?;
            }
        }
        Some(())
    }
}

impl SshDecode for CertExtension {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        Some(match &d.take_str_framed()? {
            &NO_PRESENCE_REQUIRED => {
                d.expect_str_framed("")?;
                Self::NoPresenceRequired
            }
            &PERMIT_X11_FORWARDING => {
                d.expect_str_framed("")?;
                Self::PermitX11Forwarding
            }
            &PERMIT_AGENT_FORWARDING => {
                d.expect_str_framed("")?;
                Self::PermitAgentForwarding
            }
            &PERMIT_PORT_FORWARDING => {
                d.expect_str_framed("")?;
                Self::PermitPortForwarding
            }
            &PERMIT_PTY => {
                d.expect_str_framed("")?;
                Self::PermitPty
            }
            &PERMIT_USER_RC => {
                d.expect_str_framed("")?;
                Self::PermitUserRc
            }
            name => Self::Other(String::from(*name), d.take_bytes_framed()?.into()),
        })
    }
}
