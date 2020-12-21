use super::ssh_ed25519::*;
use super::*;
use std::convert::TryFrom;

/// A public key signature.
#[derive(Clone, Debug, PartialEq)]
pub struct Signature {
    algo: String,
    data: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SignatureError {
    UnsupportedSignature,
    UnsupportedIdentity,
    InvalidSignature,
}

impl Signature {
    pub fn new(algo: String, data: Vec<u8>) -> Self {
        Self { algo, data }
    }

    pub fn algo(&self) -> &str {
        &self.algo
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn verify(&self, id: &Identity, data: &[u8]) -> Result<(), SignatureError> {
        let e = SignatureError::InvalidSignature;
        match self.algo.as_str() {
            SshEd25519::NAME => {
                use ed25519_dalek::PublicKey as PK;
                use ed25519_dalek::Signature as SG;
                let key = if let Some(id) = id.as_ssh_ed25519() {
                    PK::from_bytes(id.pk().as_ref()).map_err(|_| e)?
                } else if let Some(id) = id.as_ssh_ed25519_cert() {
                    PK::from_bytes(id.pk().as_ref()).map_err(|_| e)?
                } else {
                    return Err(SignatureError::UnsupportedIdentity);
                };
                let sig = SG::try_from(self.data.as_ref()).map_err(|_| e)?;
                key.verify_strict(data, &sig).map_err(|_| e)
            }
            _ => Err(SignatureError::UnsupportedSignature),
        }
    }
}

// FIXME: Double framing?
impl Encode for Signature {
    fn size(&self) -> usize {
        12 + self.algo.len() + self.data.len()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        let alen = self.algo.len() as u32;
        let slen = self.data.len() as u32;
        e.push_u32be(8 + alen + slen)?;
        e.push_str_framed(&self.algo)?;
        e.push_bytes_framed(&self.data)
    }
}

impl Decode for Signature {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let len = d.take_u32be()?;
        let innr = d.take_bytes(len as usize)?;
        let innr = &mut SliceDecoder::new(innr);
        let algo = Decode::decode(innr)?;
        let data = DecodeRef::decode(innr).map(|x: &[u8]| Vec::from(x))?;
        innr.expect_eoi()?;
        Some(Self { algo, data })
    }
}
