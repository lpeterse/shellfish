use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl AuthMethod for PasswordMethod {
    const NAME: &'static str = "password";
}

impl SshEncode for PasswordMethod {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bool(false)?;
        e.push_str_framed(&self.0)
    }
}

impl<'a> SshDecodeRef<'a> for PasswordMethod {
    fn decode<D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        d.expect_false()?;
        d.take().map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_01() {
        let msg = PasswordMethod("abcd".into());
        assert_eq!("PasswordMethod(\"abcd\")", format!("{:?}", msg));
    }

    #[test]
    fn test_encode_01() {
        let msg = PasswordMethod("abcd".into());
        assert_eq!(
            &[0, 0, 0, 0, 4, 97, 98, 99, 100][..],
            &SshCodec::encode(&msg).unwrap()[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 9] = [0, 0, 0, 0, 4, 97, 98, 99, 100];
        let msg: PasswordMethod = SshCodec::decode(&buf[..]).unwrap();
        assert_eq!(msg.0, "abcd");
    }
}
