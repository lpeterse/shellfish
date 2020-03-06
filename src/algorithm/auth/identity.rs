use super::*;

/// A user's or host's identity.
///
/// This is either just a key or a certificate.
/// An identity is `Unknown` if it is not implemented and cannot be interpreted by us.
#[derive(Clone, Debug, PartialEq)]
pub enum Identity {
    Ed25519Cert(<SshEd25519Cert as AuthAlgorithm>::AuthIdentity),
    Ed25519Key(<SshEd25519 as AuthAlgorithm>::AuthIdentity),
    RsaKey(<SshRsa as AuthAlgorithm>::AuthIdentity),
    Other(String),
}

impl Identity {
    /// For a given identity, yield the corresponding algorithm name.
    ///
    /// NOTE: This is implies that the relation is a bijection which might turn out as a wrong
    /// assumption in the future. Feel free to fix this as necessary.
    pub fn algorithm(&self) -> &str {
        match self {
            Self::Ed25519Cert(_) => <SshEd25519Cert as AuthAlgorithm>::NAME,
            Self::Ed25519Key(_) => <SshEd25519 as AuthAlgorithm>::NAME,
            Self::RsaKey(_) => <SshRsa as AuthAlgorithm>::NAME,
            Self::Other(x) => x.as_str(),
        }
    }

    pub fn is_valid_cert(&self, cakey: &PublicKey) -> bool {
        match (self, cakey) {
            (Self::Ed25519Cert(_), PublicKey::Ed25519(_)) => false, // FIXME
            _ => false,
        }
    }

    pub fn is_pubkey(&self, pubkey: &PublicKey) -> bool {
        match (self, pubkey) {
            (Self::Ed25519Key(x), PublicKey::Ed25519(y)) => x == y,
            //(Self::RsaKey(x), Self::RsaKey(y)) => x == y, FIXME
            _ => false,
        }
    }
}

impl Encode for Identity {
    fn size(&self) -> usize {
        match self {
            Self::Ed25519Cert(x) => Encode::size(x),
            Self::Ed25519Key(x) => Encode::size(x),
            Self::RsaKey(x) => Encode::size(x),
            Self::Other(_) => 0,
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::Ed25519Cert(x) => Encode::encode(x, e),
            Self::Ed25519Key(x) => Encode::encode(x, e),
            Self::RsaKey(x) => Encode::encode(x, e),
            Self::Other(_) => (),
        }
    }
}

impl Decode for Identity {
    // FIXME
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        None.or_else(|| {
            let mut d_ = d.clone();
            let r = d_.isolate_u32be(|x| DecodeRef::decode(x).map(Self::Ed25519Key));
            if r.is_some() {
                *d = d_;
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Self::Ed25519Cert);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = DecodeRef::decode(&mut d_).map(Self::RsaKey);
            if r.is_some() {
                *d = d_
            };
            r
        })
        .or_else(|| {
            let mut d_ = d.clone();
            let r = d_.isolate_u32be(|x| {
                let algo = DecodeRef::decode(x)?;
                let _ = x.take_all()?;
                Some(Self::Other(algo))
            });
            if r.is_some() {
                *d = d_
            };
            r
        })
    }
}
