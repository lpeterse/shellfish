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

impl Encode for Signature {
    fn size(&self) -> usize {
        12 + self.algo.len() + self.data.len()
    }
    fn encode<E: Encoder>(&self, e: &mut E) -> Option<()> {
        let alen = self.algo.len() as u32;
        let slen = self.data.len() as u32;
        e.push_u32be(8 + alen + slen)?;
        e.push_u32be(alen)?;
        e.push_bytes(&self.algo.as_bytes())?;
        e.push_u32be(slen)?;
        e.push_bytes(&self.data.as_slice())
    }
}

impl Decode for Signature {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        d.isolate_u32be(|x| {
            Some(Self {
                algo: Decode::decode(x)?,
                data: {
                    let bytes = x.take_u32be()?;
                    x.take_bytes(bytes as usize)?.into()
                },
            })
        })
    }
}
