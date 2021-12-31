mod chacha20_poly1305;
mod plain;

use self::chacha20_poly1305::*;
use self::plain::*;
use super::super::keys::*;
use super::super::*;
use super::*;

pub trait EncryptionAlgorithm {
    const NAME: &'static str;
}

pub struct Chacha20Poly1305AtOpensshDotCom;

impl EncryptionAlgorithm for Chacha20Poly1305AtOpensshDotCom {
    const NAME: &'static str = "chacha20-poly1305@openssh.com";
}

#[derive(Clone, Debug)]
pub struct CipherConfig {
    /// Encryption algorithm
    pub ea: &'static str,
    /// Compression algorithm
    pub ca: &'static str,
    /// MAC algorithm
    pub ma: Option<&'static str>,
    /// Encryption key stream
    pub ke: KeyStream,
}

impl CipherConfig {
    pub fn new(
        ea: &'static str,
        ca: &'static str,
        ma: Option<&'static str>,
        ke: KeyStream,
    ) -> Self {
        Self { ea, ca, ma, ke }
    }
}

#[derive(Debug)]
pub enum CipherContext {
    Plain(PlainContext),
    Chacha20Poly1305(Chacha20Poly1305Context),
}

impl CipherContext {
    pub fn new() -> Self {
        Self::Plain(PlainContext::new())
    }

    pub fn update(&mut self, mut config: Box<CipherConfig>) -> Result<(), TransportError> {
        match (config.ea, config.ca, config.ma) {
            (Chacha20Poly1305AtOpensshDotCom::NAME, NoCompression::NAME, None) => {
                match self {
                    // Just pass new keys to existing instance (very likely)
                    Self::Chacha20Poly1305(ctx) => ctx.update(&mut config.ke),
                    // Create and assign new instance
                    _ => {
                        *self = Self::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut config.ke))
                    }
                }
            }
            _ => return Err(TransportError::NoCommonEncryptionAlgorithm),
        }
        Ok(())
    }

    pub fn encrypt(&self, pc: u64, buf: &mut [u8]) -> Result<(), TransportError> {
        match self {
            Self::Plain(ctx) => ctx.encrypt(pc, buf),
            Self::Chacha20Poly1305(ctx) => ctx.encrypt(pc, buf),
        }
    }

    pub fn decrypt(&self, pc: u64, buf: &mut [u8]) -> Result<(), TransportError> {
        match self {
            Self::Plain(ctx) => ctx.decrypt(pc, buf),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt(pc, buf),
        }
    }

    pub fn decrypt_len(&self, pc: u64, len: [u8; 4]) -> Result<usize, TransportError> {
        match self {
            Self::Plain(ctx) => ctx.decrypt_len(pc, len),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt_len(pc, len),
        }
    }

    pub fn mac_len(&self) -> usize {
        match self {
            Self::Plain(_) => PlainContext::MAC_LEN,
            Self::Chacha20Poly1305(_) => Chacha20Poly1305Context::MAC_LEN,
        }
    }

    pub fn padding_len(&self, payload_len: usize) -> usize {
        match self {
            Self::Plain(ctx) => ctx.padding_len(payload_len),
            Self::Chacha20Poly1305(ctx) => ctx.padding_len(payload_len),
        }
    }
}
