use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl<'a> AuthMethod for PasswordMethod {
    const NAME: &'static str = "password";
}

impl Encode for PasswordMethod {
    fn size(&self) -> usize {
        4 + self.0.len()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_str_framed(&self.0)
    }
}

impl<'a> DecodeRef<'a> for PasswordMethod {
    fn decode<D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        DecodeRef::decode(d).map(PasswordMethod)
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
            &[0, 0, 0, 4, 97, 98, 99, 100][..],
            &SliceEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 8] = [0, 0, 0, 4, 97, 98, 99, 100];
        let msg: PasswordMethod = SliceDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.0, "abcd");
    }
}
