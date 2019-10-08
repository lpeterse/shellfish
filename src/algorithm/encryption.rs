pub trait EncryptionAlgorithm {
    const NAME: &'static str;
}

pub struct Chacha20Poly1305AtOpensshDotCom {}

impl EncryptionAlgorithm for Chacha20Poly1305AtOpensshDotCom {
    const NAME: &'static str = "chacha20-poly1305@openssh.com";
}
