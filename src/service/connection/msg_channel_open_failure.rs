use crate::codec::*;
use crate::message::*;

use std::fmt;

#[derive(Clone, Debug)]
pub struct MsgChannelOpenFailure {
    pub recipient_channel: u32,
    pub reason: Reason,
    pub description: String,
    pub language: String,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Reason(u32);

impl Reason {
    pub const ADMINISTRATIVELY_PROHIBITED: Self = Self(1);
    pub const OPEN_CONNECT_FAILED: Self = Self(2);
    pub const UNKNOWN_CHANNEL_TYPE: Self = Self(3);
    pub const RESOURCE_SHORTAGE: Self = Self(4);
}

impl fmt::Debug for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            &Self::ADMINISTRATIVELY_PROHIBITED => write!(f, "Reason::ADMINISTRATIVELY_PROHIBITED"),
            &Self::OPEN_CONNECT_FAILED => write!(f, "Reason::OPEN_CONNECT_FAILED"),
            &Self::UNKNOWN_CHANNEL_TYPE => write!(f, "Reason::UNKNOWN_CHANNEL_TYPE"),
            &Self::RESOURCE_SHORTAGE => write!(f, "Reason::RESOURCE_SHORTAGE"),
            _ => write!(f, "Reason({})", self.0),
        }
    }
}

impl<'a> Message for MsgChannelOpenFailure {
    const NUMBER: u8 = 92;
}

impl Encode for MsgChannelOpenFailure {
    fn size(&self) -> usize {
        1 + 4 + 4 + Encode::size(&self.description) + Encode::size(&self.language)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        e.push_u32be(self.recipient_channel);
        e.push_u32be(self.reason.0);
        Encode::encode(&self.description, e);
        Encode::encode(&self.language, e);
    }
}

impl<'a> DecodeRef<'a> for MsgChannelOpenFailure {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(Self::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            reason: Reason(d.take_u32be()?),
            description: DecodeRef::decode(d)?,
            language: DecodeRef::decode(d)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_reason_01() {
        assert_eq!(format!("{:?}", Reason::ADMINISTRATIVELY_PROHIBITED), "Reason::ADMINISTRATIVELY_PROHIBITED");
        assert_eq!(format!("{:?}", Reason::OPEN_CONNECT_FAILED), "Reason::OPEN_CONNECT_FAILED");
        assert_eq!(format!("{:?}", Reason::UNKNOWN_CHANNEL_TYPE), "Reason::UNKNOWN_CHANNEL_TYPE");
        assert_eq!(format!("{:?}", Reason::RESOURCE_SHORTAGE), "Reason::RESOURCE_SHORTAGE");
        assert_eq!(format!("{:?}", Reason(5)), "Reason(5)");
    }

    #[test]
    fn test_debug_01() {
        let msg = MsgChannelOpenFailure {
            recipient_channel: 23,
            reason: Reason::ADMINISTRATIVELY_PROHIBITED,
            description: "desc".into(),
            language: "lang".into(),
        };
        assert_eq!(
            "MsgChannelOpenFailure { recipient_channel: 23, reason: Reason::ADMINISTRATIVELY_PROHIBITED, description: \"desc\", language: \"lang\" }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgChannelOpenFailure {
            recipient_channel: 23,
            reason: Reason::ADMINISTRATIVELY_PROHIBITED,
            description: "desc".into(),
            language: "lang".into(),
        };
        assert_eq!(
            &[
                92, 0, 0, 0, 23, 0, 0, 0, 1, 0, 0, 0, 4, 100, 101, 115, 99, 0, 0, 0, 4, 108, 97,
                110, 103
            ][..],
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 25] = [
            92, 0, 0, 0, 23, 0, 0, 0, 1, 0, 0, 0, 4, 100, 101, 115, 99, 0, 0, 0, 4, 108, 97, 110,
            103,
        ];
        let msg: MsgChannelOpenFailure = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.recipient_channel, 23);
        assert_eq!(
            msg.reason,
            Reason::ADMINISTRATIVELY_PROHIBITED
        );
        assert_eq!(msg.description, "desc");
        assert_eq!(msg.language, "lang");
    }
}
