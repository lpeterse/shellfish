pub trait KexAlgorithm {
    const NAME: &'static str;
}

pub struct Curve25519Sha256 {}

impl KexAlgorithm for Curve25519Sha256 {
    const NAME: &'static str = "curve25519-sha256";
}

pub(crate) const KEX_ALGORITHMS: [&'static str; 1] = [<Curve25519Sha256 as KexAlgorithm>::NAME];
