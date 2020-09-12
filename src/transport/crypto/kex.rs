pub trait KexAlgorithm {
    const NAME: &'static str;
}

pub struct Curve25519Sha256 {}

impl KexAlgorithm for Curve25519Sha256 {
    const NAME: &'static str = "curve25519-sha256";
}

pub struct Curve25519Sha256AtLibsshDotOrg {}

impl KexAlgorithm for Curve25519Sha256AtLibsshDotOrg {
    const NAME: &'static str = "curve25519-sha256@libssh.org";
}

pub(crate) const KEX_ALGORITHMS: [&'static str; 2] = [
    <Curve25519Sha256 as KexAlgorithm>::NAME,
    <Curve25519Sha256AtLibsshDotOrg as KexAlgorithm>::NAME,
];
