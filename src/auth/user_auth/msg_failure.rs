use crate::util::codec::*;
use crate::transport::Message;

#[derive(Clone, Debug)]
pub struct MsgFailure<'a> {
    pub methods: Vec<&'a str>,
    pub partial_success: bool,
}

impl<'a> Message for MsgFailure<'a> {
    const NUMBER: u8 = 51;
}

impl<'a> Encode for MsgFailure<'a> {
    fn size(&self) -> usize {
        1 + NameList::size(&self.methods) + 1
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        NameList::encode(&self.methods, e)?;
        e.push_u8(self.partial_success as u8)
    }
}

impl<'a> DecodeRef<'a> for MsgFailure<'a> {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.take_u8().filter(|x| x == &<Self as Message>::NUMBER)?;
        Self {
            methods: NameList::decode_str(d)?,
            partial_success: d.take_u8().map(|x| x != 0)?,
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = MsgFailure {
            methods: vec!["password", "publickey"],
            partial_success: true,
        };
        assert_eq!(
            "MsgFailure { methods: [\"password\", \"publickey\"], partial_success: true }",
            format!("{:?}", msg)
        );
    }

    #[test]
    fn test_encode_01() {
        let msg = MsgFailure {
            methods: vec!["password", "publickey"],
            partial_success: true,
        };
        assert_eq!(
            &[
                51, 0, 0, 0, 18, 112, 97, 115, 115, 119, 111, 114, 100, 44, 112, 117, 98, 108, 105,
                99, 107, 101, 121, 1
            ][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgFailure {
            methods: vec![],
            partial_success: false,
        };
        assert_eq!(
            &[51, 0, 0, 0, 0, 0][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 24] = [
            51, 0, 0, 0, 18, 112, 97, 115, 115, 119, 111, 114, 100, 44, 112, 117, 98, 108, 105, 99,
            107, 101, 121, 1,
        ];
        let msg: MsgFailure = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.methods, vec!["password", "publickey"]);
        assert_eq!(msg.partial_success, true);
    }

    #[test]
    fn test_decode_02() {
        let buf: [u8; 6] = [51, 0, 0, 0, 0, 0];
        let msg: MsgFailure = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.methods.is_empty(), true);
        assert_eq!(msg.partial_success, false);
    }
}
