use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum PublicKey {
    Ed25519(SshEd25519PublicKey), // FIXME: Add other algos
    Rsa(SshRsaPublicKey),
    Other(String)
}

impl PublicKey {
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519(_) => <SshEd25519 as AuthAlgorithm>::NAME,
            Self::Rsa(_) => <SshRsa as AuthAlgorithm>::NAME,
            Self::Other(x) => x.as_str(),
        }
    }

    pub fn is_pubkey(&self, pubkey: &PublicKey) -> bool {
        match (self, pubkey) {
            (Self::Ed25519(x), Self::Ed25519(y)) => x == y,
            (Self::Rsa(x), Self::Rsa(y)) => x == y, 
            _ => false,
        }
    }

    pub fn decode<'a, D: Decoder<'a>>(d: &mut D, algo: &str) -> Option<PublicKey> {
        match algo {
            <SshEd25519 as AuthAlgorithm>::NAME => {
                Decode::decode(d).map(PublicKey::Ed25519)
            }
            _ => None,
        }
    }
}

impl Encode for PublicKey {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519(x) => Encode::size(x),
            Self::Rsa(x) => Encode::size(x),
            Self::Other(_) => 0,
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::Ed25519(x) => Encode::encode(x, e),
            Self::Rsa(x) => Encode::encode(x, e),
            Self::Other(_) => (),
        }
    }
}

impl Decode for PublicKey {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let mut d_ = d.clone();
        if let Some(r) = d_.isolate_u32be(|x| DecodeRef::decode(x)) {
            *d = d_;
            return Some(Self::Ed25519(r))
        }
        let mut d_ = d.clone();
        if let Some(r) = DecodeRef::decode(&mut d_) {
            *d = d_;
            return Some(Self::Rsa(r))
        }
        d.isolate_u32be(|x| {
            let algo = Decode::decode(x)?;
            let _ = x.take_all()?;
            Some(Self::Other(algo))
        })
    }
}
