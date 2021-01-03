use super::super::disconnect::DisconnectReason;
use super::Message;
use crate::util::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MsgDisconnect<'a> {
    pub reason: DisconnectReason,
    pub description: &'a str,
    pub language: &'a str,
}

impl<'a> MsgDisconnect<'a> {
    pub fn new(reason: DisconnectReason) -> Self {
        Self {
            reason,
            description: "",
            language: "",
        }
    }
}

impl<'a> Message for MsgDisconnect<'a> {
    const NUMBER: u8 = 1;
}

impl<'a> SshEncode for MsgDisconnect<'a> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.reason.0)?;
        e.push_str_framed(&self.description)?;
        e.push_str_framed(&self.language)
    }
}

impl<'a> SshDecodeRef<'a> for MsgDisconnect<'a> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Some(Self {
            reason: d.take_u32be().map(DisconnectReason)?,
            description: d.take_str_framed()?,
            language: d.take_str_framed()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg = MsgDisconnect::new(DisconnectReason::MAC_ERROR);
        assert_eq!(
            &[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        assert_eq!(
            &Some(MsgDisconnect::new(DisconnectReason::MAC_ERROR)),
            &SshCodec::decode(&[1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0][..])
        );
    }
}
