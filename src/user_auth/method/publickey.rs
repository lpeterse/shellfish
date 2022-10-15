use super::*;
use crate::identity::*;

#[derive(Debug)]
pub struct PublicKeyMethod {
    pub algorithm: String,
    pub identity: Identity,
    pub signature: Option<Signature>,
}

impl<'a> AuthMethod for PublicKeyMethod {
    const NAME: &'static str = "publickey";
}

impl SshEncode for PublicKeyMethod {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bool(self.signature.is_some())?;
        e.push_str_framed(&self.algorithm)?;
        e.push(&self.identity)?;
        match self.signature {
            None => Some(()),
            Some(ref x) => e.push(x),
        }
    }
}

impl SshDecode for PublicKeyMethod {
    fn decode<'a, D: SshDecoder<'a>>(d: &mut D) -> Option<Self> {
        let signature = d.take_bool()?;
        let algorithm = d.take_str_framed()?.into();
        let identity = d.take()?;
        let signature = if signature {
            Some(d.take()?)
        } else {
            None
        };
        Some(Self {
            algorithm,
            identity,
            signature
        })
    }
}
