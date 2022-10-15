use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgFailure<T = String> {
    pub methods: Vec<T>,
    pub partial_success: bool,
}

impl <T> Message for MsgFailure<T> {
    const NUMBER: u8 = 51;
}

impl <T> MsgFailure<T> {
    pub fn new(partial_success: bool, methods: Vec<T>) -> Self {
        Self {
            methods,
            partial_success
        }
    }
}

impl SshEncode for MsgFailure<&'static str> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(MsgFailure::<String>::NUMBER)?;
        e.push_name_list(&self.methods)?;
        e.push_bool(self.partial_success)
    }
}

impl SshDecode for MsgFailure {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_u8(<Self as Message>::NUMBER)?;
        let methods = d.take_name_list()?.filter(|x| !x.is_empty());
        let methods = methods.map(Into::into).collect();
        let partial_success = d.take_bool()?;
        Some(Self {
            methods,
            partial_success,
        })
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
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_encode_02() {
        let msg = MsgFailure {
            methods: vec![],
            partial_success: false,
        };
        assert_eq!(&[51, 0, 0, 0, 0, 0][..], &SshCodec::encode(&msg).unwrap()[..]);
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 24] = [
            51, 0, 0, 0, 18, 112, 97, 115, 115, 119, 111, 114, 100, 44, 112, 117, 98, 108, 105, 99,
            107, 101, 121, 1,
        ];
        let msg: MsgFailure = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.methods, vec!["password", "publickey"]);
        assert_eq!(msg.partial_success, true);
    }

    #[test]
    fn test_decode_02() {
        let buf: [u8; 6] = [51, 0, 0, 0, 0, 0];
        let msg: MsgFailure = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.methods.is_empty(), true);
        assert_eq!(msg.partial_success, false);
    }
}
