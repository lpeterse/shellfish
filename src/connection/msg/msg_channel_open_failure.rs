use crate::transport::Message;
use crate::util::codec::*;
use super::super::channel::OpenFailure;

#[derive(Clone, Debug)]
pub(crate) struct MsgOpenFailure {
    pub recipient_channel: u32,
    pub reason: OpenFailure,
    pub description: String,
    pub language: String,
}

impl MsgOpenFailure {
    pub fn new(recipient_channel: u32, reason: OpenFailure) -> Self {
        Self {
            recipient_channel,
            reason,
            description: "".into(),
            language: "".into(),
        }
    }
}

impl<'a> Message for MsgOpenFailure {
    const NUMBER: u8 = 92;
}

impl SshEncode for MsgOpenFailure {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_u32be(self.reason.0)?;
        e.push_str_framed(&self.description)?;
        e.push_str_framed(&self.language)
    }
}

impl<'a> SshDecodeRef<'a> for MsgOpenFailure {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::NUMBER)?;
        Some(Self {
            recipient_channel: d.take_u32be()?,
            reason: d.take_u32be().map(OpenFailure)?,
            description: d.take_str_framed()?.into(),
            language: d.take_str_framed()?.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_reason_01() {
        assert_eq!(
            format!("{:?}", OpenFailure::ADMINISTRATIVELY_PROHIBITED),
            "OpenFailure::ADMINISTRATIVELY_PROHIBITED"
        );
        assert_eq!(
            format!("{:?}", OpenFailure::OPEN_CONNECT_FAILED),
            "OpenFailure::OPEN_CONNECT_FAILED"
        );
        assert_eq!(
            format!("{:?}", OpenFailure::UNKNOWN_CHANNEL_TYPE),
            "OpenFailure::UNKNOWN_CHANNEL_TYPE"
        );
        assert_eq!(
            format!("{:?}", OpenFailure::RESOURCE_SHORTAGE),
            "OpenFailure::RESOURCE_SHORTAGE"
        );
        assert_eq!(
            format!("{:?}", OpenFailure(5)),
            "OpenFailure(5)"
        );
    }

    #[test]
    fn test_debug_01() {
        let msg = MsgOpenFailure {
            recipient_channel: 23,
            reason: OpenFailure::ADMINISTRATIVELY_PROHIBITED,
            description: "desc".into(),
            language: "lang".into(),
        };
        assert_eq!(
            "MsgOpenFailure { recipient_channel: 23, reason: OpenFailure::ADMINISTRATIVELY_PROHIBITED, description: \"desc\", language: \"lang\" }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgOpenFailure {
            recipient_channel: 23,
            reason: OpenFailure::ADMINISTRATIVELY_PROHIBITED,
            description: "desc".into(),
            language: "lang".into(),
        };
        assert_eq!(
            &[
                92, 0, 0, 0, 23, 0, 0, 0, 1, 0, 0, 0, 4, 100, 101, 115, 99, 0, 0, 0, 4, 108, 97,
                110, 103
            ][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 25] = [
            92, 0, 0, 0, 23, 0, 0, 0, 1, 0, 0, 0, 4, 100, 101, 115, 99, 0, 0, 0, 4, 108, 97, 110,
            103,
        ];
        let msg: MsgOpenFailure = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(msg.reason, OpenFailure::ADMINISTRATIVELY_PROHIBITED);
        assert_eq!(msg.description, "desc");
        assert_eq!(msg.language, "lang");
    }
}
