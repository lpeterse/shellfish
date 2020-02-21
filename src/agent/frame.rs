use crate::codec::*;

pub struct Frame<T> {
    payload: T,
}

impl<T> Frame<T> {
    pub fn new(payload: T) -> Self {
        Self { payload }
    }
}

impl<T: Encode> Encode for Frame<T> {
    fn size(&self) -> usize {
        4 + self.payload.size()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u32be(self.payload.size() as u32);
        self.payload.encode(e);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_01() {
        let data: &'static str = "data";
        let msg: Frame<_> = Frame::new(data);
        assert_eq!(Encode::size(&msg), 12);
        assert_eq!(
            BEncoder::encode(&msg),
            vec![0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97]
        );
    }
}
