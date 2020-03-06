use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Certificate {
    Ed25519(<SshEd25519Cert as AuthAlgorithm>::AuthIdentity),
}

impl Certificate {
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519(_) => <SshEd25519Cert as AuthAlgorithm>::NAME,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::Ed25519(x) => x.public_key()
        }
    }
}

impl Encode for Certificate {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::Ed25519(x) => Encode::encode(x, e),
        }
    }
}

impl Decode for Certificate {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        DecodeRef::decode(d).map(Self::Ed25519)
    }
}
