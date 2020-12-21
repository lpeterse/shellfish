use crate::util::codec::*;

pub struct Frame<'a, T>(pub &'a T);

impl<'a, T: Encode> Encode for Frame<'a, T> {
    fn size(&self) -> usize {
        4 + self.0.size()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_usize(self.0.size())?;
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
        assert_eq!(Encode::size(&msg), 12);
        assert_eq!(
            SliceEncoder::encode(&msg),
            vec![0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97]
        );
    }
}
