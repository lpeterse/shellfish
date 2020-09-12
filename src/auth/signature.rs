use super::*;

/// A public key signature.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    pub algorithm: String,
    pub signature: Vec<u8>,
}

impl Encode for Signature {
    fn size(&self) -> usize {
        12 + self.algorithm.len() + self.signature.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        let alen = self.algorithm.len() as u32;
        let slen = self.signature.len() as u32;
        e.push_u32be(8 + alen + slen)?;
        e.push_u32be(alen)?;
        e.push_bytes(&self.algorithm.as_bytes())?;
        e.push_u32be(slen)?;
        e.push_bytes(&self.signature.as_slice())
    }
}

impl Decode for Signature {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.isolate_u32be(|x| {
            Some(Self {
                algorithm: Decode::decode(x)?,
                signature: {
                    let bytes = x.take_u32be()?;
                    x.take_bytes(bytes as usize)?.into()
                },
            })
        })
    }
}
