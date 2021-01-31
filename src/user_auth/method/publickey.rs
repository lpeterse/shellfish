use super::*;
use crate::identity::*;

#[derive(Debug)]
pub struct PublicKeyMethod {
    pub identity: Identity,
    pub signature: Option<Signature>,
}

impl<'a> AuthMethod for PublicKeyMethod {
    const NAME: &'static str = "publickey";
}

impl SshEncode for PublicKeyMethod {
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_bool(self.signature.is_some())?;
        e.push_str_framed(self.identity.algorithm())?;
        e.push(&self.identity)?;
        match self.signature {
            None => Some(()),
            Some(ref x) => e.push(x),
        }
    }
}
