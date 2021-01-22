use crate::util::codec::*;

pub struct Frame<'a, T>(pub &'a T);

impl<'a, T: SshEncode> SshEncode for Frame<'a, T> {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_usize(SshCodec::size(self.0)?)?;
        e.push(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let data = String::from("data");
        let msg: Frame<_> = Frame(&data);
        assert_eq!(
            SshCodec::encode(&msg).unwrap(),
            vec![0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97]
        );
    }
}
