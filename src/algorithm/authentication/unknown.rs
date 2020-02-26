use crate::codec::*;

#[derive(Clone, Debug, PartialEq)]
pub struct UnknownIdentity {
    pub algo: String,
    pub data: Vec<u8>,
}

impl Encode for UnknownIdentity {
    fn size(&self) -> usize {
        Encode::size(&self.algo) + Encode::size(&self.data[..])
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        Encode::encode(&self.algo, e);
        Encode::encode(&self.data[..], e);
    }
}

impl Decode for UnknownIdentity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        Self {
            algo: Decode::decode(d)?,
            data: DecodeRef::decode(d).map(|x: &'a [u8]| Vec::from(x))?,
        }
        .into()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_01() {
        let msg: UnknownIdentity = UnknownIdentity {
            algo: "ssh-fobar".into(),
            data: vec![1, 2, 3, 4],
        };
        assert_eq!(
            vec![0, 0, 0, 9, 115, 115, 104, 45, 102, 111, 98, 97, 114, 0, 0, 0, 4, 1, 2, 3, 4],
            BEncoder::encode(&msg)
        );
    }

    #[test]
    fn test_decode_01() {
        let msg: UnknownIdentity = UnknownIdentity {
            algo: "ssh-fobar".into(),
            data: vec![1, 2, 3, 4],
        };
        assert_eq!(
            Some(msg),
            BDecoder::decode(
                &[0, 0, 0, 9, 115, 115, 104, 45, 102, 111, 98, 97, 114, 0, 0, 0, 4, 1, 2, 3, 4][..]
            )
        );
    }
}
