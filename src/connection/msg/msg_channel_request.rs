use crate::connection::{ExitStatus, ExitSignal};
use crate::transport::Message;
use crate::util::codec::*;

#[derive(Debug)]
pub(crate) struct MsgChannelRequest<'a, T> {
    pub recipient_channel: u32,
    pub request: &'a str,
    pub want_reply: bool,
    pub specific: T,
}

impl <'a> MsgChannelRequest<'a, &'a ExitStatus> {
    pub fn new_exit_status(rid: u32, status: &'a ExitStatus) -> Self {
        Self {
            recipient_channel: rid,
            request: "exit-status",
            want_reply: false,
            specific: status
        }
    }
}

impl <'a> MsgChannelRequest<'a, &'a ExitSignal> {
    pub fn new_exit_signal(rid: u32, signal: &'a ExitSignal) -> Self {
        Self {
            recipient_channel: rid,
            request: "exit-status",
            want_reply: false,
            specific: signal
        }
    }
}

impl<'a, T> Message for MsgChannelRequest<'a, T> {
    const NUMBER: u8 = 98;
}

impl<'a, T: SshEncode> SshEncode for MsgChannelRequest<'a, T> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER)?;
        e.push_u32be(self.recipient_channel)?;
        e.push_str_framed(&self.request)?;
        e.push_bool(self.want_reply)?;
        e.push(&self.specific)
    }
}

impl<'a> SshDecodeRef<'a> for MsgChannelRequest<'a, &'a [u8]> {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        Self {
            recipient_channel: d.take_u32be()?,
            request: d.take_str_framed()?,
            want_reply: d.take_bool()?,
            specific: d.take_bytes_all()?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let x = MsgChannelRequest {
            recipient_channel: 23,
            request: "request",
            want_reply: true,
            specific: "specific",
        };
        assert_eq!(format!("{:?}", x), "MsgChannelRequest { recipient_channel: 23, request: \"request\", want_reply: true, specific: \"specific\" }");
    }

    #[test]
    fn test_encode_01() {
        let x = MsgChannelRequest {
            recipient_channel: 23,
            request: "request",
            want_reply: true,
            specific: (),
        };
        let actual = SshCodec::encode(&x).unwrap();
        let expected = [
            98, 0, 0, 0, 23, 0, 0, 0, 7, 114, 101, 113, 117, 101, 115, 116, 1,
        ];
        assert_eq!(&actual[..], &expected[..]);
    }

    #[test]
    fn test_decode_01() {
        let x: MsgChannelRequest<&[u8]> = SshCodec::decode(
            &[
                98, 0, 0, 0, 23, 0, 0, 0, 7, 114, 101, 113, 117, 101, 115, 116, 1, 0, 0, 0, 8, 115,
                112, 101, 99, 105, 102, 105, 99,
            ][..],
        )
        .unwrap();
        assert_eq!(x.recipient_channel, 23);
        assert_eq!(x.request, "request");
        assert_eq!(x.want_reply, true);
    }
}
