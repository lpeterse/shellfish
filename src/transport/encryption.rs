mod chacha20_poly1305;
mod plain;

use self::chacha20_poly1305::*;
use self::plain::*;
use super::*;
use crate::algorithm::*;

pub enum EncryptionContext {
    Plain(PlainEncryptionContext),
    Chacha20Poly1305(Chacha20Poly1305EncryptionContext),
}

impl EncryptionContext {
    pub fn new() -> Self {
        Self::Plain(PlainEncryptionContext::new())
    }

    pub fn new_keys(
        &mut self,
        enc: &EncryptionAlgorithm,
        comp: &CompressionAlgorithm,
        _mac: &Option<MacAlgorithm>,
        ks: &mut KeyStream,
    ) {
        match (enc, comp) {
            // chacha20-poly1305@openssh.com ignores the mac algorithm
            (EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom, CompressionAlgorithm::None) => {
                match self {
                    // Just pass new keys to existing instance (very likely)
                    Self::Chacha20Poly1305(ctx) => ctx.new_keys(ks),
                    // Create and assign new instance
                    _ => *self = Self::Chacha20Poly1305(Chacha20Poly1305EncryptionContext::new(ks)),
                }
            }
            algos => panic!("Algorithms not supported: {:?}", algos),
        }
    }

    pub fn buffer_layout(&self, payload_len: usize) -> PacketLayout {
        match self {
            Self::Plain(ctx) => ctx.buffer_layout(payload_len),
            Self::Chacha20Poly1305(ctx) => ctx.buffer_layout(payload_len),
        }
    }

    pub fn encrypt_packet(&self, pc: u64, layout: PacketLayout, buf: &mut [u8]) {
        match self {
            Self::Plain(ctx) => ctx.encrypt_packet(pc, layout, buf),
            Self::Chacha20Poly1305(ctx) => ctx.encrypt_packet(pc, layout, buf),
        }
    }

    pub fn decrypt_len(&self, pc: u64, len: [u8; 4]) -> Option<usize> {
        match self {
            Self::Plain(ctx) => ctx.decrypt_len(pc, len),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt_len(pc, len),
        }
    }

    pub fn decrypt_packet(&self, pc: u64, buf: &mut [u8]) -> Option<usize> {
        match self {
            Self::Plain(ctx) => ctx.decrypt_packet(pc, buf),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt_packet(pc, buf),
        }
    }

    pub fn supported_encryption_algorithms() -> &'static [EncryptionAlgorithm] {
        &[EncryptionAlgorithm::Chacha20Poly1305AtOpensshDotCom]
    }

    pub fn supported_mac_algorithms() -> &'static [MacAlgorithm] {
        &[]
    }
}
