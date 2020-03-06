use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum PublicKey {
    Ed25519(SshEd25519PublicKey), // FIXME: Add other algos
    Unknown(String)
}

impl PublicKey {
    pub fn decode<'a, D: Decoder<'a>>(d: &mut D, algo: &str) -> Option<PublicKey> {
        match algo {
            <SshEd25519 as AuthAlgorithm>::NAME => {
                Decode::decode(d).map(PublicKey::Ed25519)
            }
            _ => None,
        }
    }
}
