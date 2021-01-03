mod compression;
mod encryption;
mod kex;

pub use self::compression::*;
pub use self::encryption::*;
pub use self::kex::*;

pub(crate) const MAC_ALGORITHMS: [&'static str; 0] = [];

pub(crate) const COMPRESSION_ALGORITHMS: [&'static str; 1] =
    [<self::compression::NoCompression as CompressionAlgorithm>::NAME];

pub(crate) const ENCRYPTION_ALGORITHMS: [&'static str; 1] =
    [<self::encryption::Chacha20Poly1305AtOpensshDotCom as EncryptionAlgorithm>::NAME];
