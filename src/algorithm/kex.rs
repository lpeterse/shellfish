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
