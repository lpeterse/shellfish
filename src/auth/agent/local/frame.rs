use crate::util::codec::*;

pub struct Frame<'a, T>(pub &'a T);

/*
impl<'a, T> Frame<'a, T> {
    pub fn new(payload: &'a T) -> Self {
        Self { payload }
    }
}
*/

impl<'a, T: Encode> Encode for Frame<'a, T> {
    fn size(&self) -> usize {
        4 + self.0.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        e.push_u32be(self.0.size() as u32)?;
        self.0.encode(e)
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
