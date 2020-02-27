use super::*;

#[derive(Debug)]
pub struct PasswordMethod(pub String);

impl<'a> AuthMethod for PasswordMethod {
    const NAME: &'static str = "password";
}

impl Encode for PasswordMethod {
    fn size(&self) -> usize {
        Encode::size(&self.0)
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.0, e);
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
            &BEncoder::encode(&msg)[..]
        );
    }

    #[test]
    fn test_decode_01() {
        let buf: [u8; 8] = [0, 0, 0, 4, 97, 98, 99, 100];
        let msg: PasswordMethod = BDecoder::decode(&buf[..]).unwrap();
        assert_eq!(msg.0, "abcd");
    }
}
