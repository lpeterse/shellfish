use super::*;

/// A user's or host's identity.
///
/// This is either just a key or a certificate.
/// An identity is `Other` if it is not implemented and cannot be interpreted by us.
#[derive(Clone, Debug, PartialEq)]
pub enum Identity {
    PublicKey(PublicKey),
    Certificate(Certificate),
}

impl Identity {
    /// For a given identity, yield the corresponding algorithm name.
    ///
    /// NOTE: This is implies that the relation is a bijection which might turn out as a wrong
    /// assumption in the future. Feel free to fix this as necessary.
    pub fn algorithm(&self) -> &str {
        match self {
            Self::PublicKey(x) => x.algorithm(),
            Self::Certificate(x) => x.algorithm(),
        }
    }

    pub fn public_key(&self) -> PublicKey {
        match self {
            Self::PublicKey(x) => x.clone(),
            Self::Certificate(x) => x.public_key(),
        }
    }

    pub fn is_valid_cert(&self, cakey: &PublicKey) -> bool {
        match (self, cakey) {
            (Self::Certificate(_), PublicKey::Ed25519(_)) => false, // FIXME
            _ => false,
        }
    }

    pub fn is_pubkey(&self, pubkey: &PublicKey) -> bool { // FIXME
        match self {
            Self::PublicKey(x) => x == pubkey,
            _ => false,
        }
    }
}

impl Encode for Identity {
    fn size(&self) -> usize {
        match self {
            Self::PublicKey(x) => Encode::size(x),
            Self::Certificate(x) => Encode::size(x),
        }
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        match self {
            Self::PublicKey(x) => Encode::encode(x, e),
            Self::Certificate(x) => Encode::encode(x, e),
        }
    }
}

impl Decode for Identity {
    fn decode<'a, D: Decoder<'a>>(d: &mut D) -> Option<Self> {
        let mut d_ = d.clone();
        if let Some(r) = Decode::decode(&mut d_) {
            *d = d_;
            return Some(Self::Certificate(r));
        }
        DecodeRef::decode(d).map(Self::PublicKey)
    }
}
