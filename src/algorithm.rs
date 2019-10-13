pub mod authentication;
pub mod compression;
pub mod encryption;
pub mod kex;

pub use self::authentication::AuthenticationAlgorithm;
pub use self::compression::CompressionAlgorithm;
pub use self::encryption::EncryptionAlgorithm;
pub use self::kex::KexAlgorithm;

pub(crate) const SUPPORTED_KEX_ALGORITHMS: [&'static str; 2] = [
    <self::kex::Curve25519Sha256 as KexAlgorithm>::NAME,
    <self::kex::Curve25519Sha256AtLibsshDotOrg as KexAlgorithm>::NAME,
];

pub(crate) const SUPPORTED_HOST_KEY_ALGORITHMS: [&'static str; 1] =
    [<self::authentication::SshEd25519 as AuthenticationAlgorithm>::NAME];

pub(crate) const SUPPORTED_MAC_ALGORITHMS: [&'static str; 0] = [];

pub(crate) const SUPPORTED_COMPRESSION_ALGORITHMS: [&'static str; 1] =
    [<self::compression::NoCompression as CompressionAlgorithm>::NAME];

pub(crate) const SUPPORTED_ENCRYPTION_ALGORITHMS: [&'static str; 1] =
    [<self::encryption::Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME];
