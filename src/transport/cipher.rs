mod chacha20_poly1305;
mod plain;

use self::chacha20_poly1305::*;
use self::plain::*;
use crate::algorithm::encryption::*;
use crate::algorithm::compression::*;
use super::*;

pub enum CipherContext {
    Plain(PlainContext),
    Chacha20Poly1305(Chacha20Poly1305Context),
}

impl CipherContext {
    pub fn new() -> Self {
        Self::Plain(PlainContext::new())
    }

    pub fn update(
        &mut self,
        enc: &'static str,
        comp: &'static str,
        mac: Option<&'static str>,
        ks: &mut KeyStream,
    ) -> Option<()> {
        match (enc, comp, mac) {
            (Chacha20Poly1305AtOpensshDotCom::NAME, NoCompression::NAME, None) => {
                match self {
                    // Just pass new keys to existing instance (very likely)
                    Self::Chacha20Poly1305(ctx) => ctx.update(ks),
                    // Create and assign new instance
                    _ => *self = Self::Chacha20Poly1305(Chacha20Poly1305Context::new(ks)),
                }
            }
            _ => return None,
        }
        Some(())
    }

    pub fn buffer_layout(&self, payload_len: usize) -> PacketLayout {
        match self {
            Self::Plain(_) => PacketLayout::new(
                payload_len,
                PlainContext::BLOCK_LEN,
                PlainContext::MAC_LEN,
            ),
            Self::Chacha20Poly1305(_) => PacketLayout::new_aad(
                payload_len,
                Chacha20Poly1305Context::BLOCK_LEN,
                Chacha20Poly1305Context::MAC_LEN,
            ),
        }
    }

    pub fn encrypt(&self, pc: u64, layout: PacketLayout, buf: &mut [u8]) {
        match self {
            Self::Plain(ctx) => {
                // Insert packet len, padding len and padding
                layout.put_len(buf);
                layout.pad_zero(buf);
                ctx.encrypt(pc, buf)
            },
            Self::Chacha20Poly1305(ctx) => {
                // Insert packet len, padding len and padding
                layout.put_len(buf);
                layout.pad_zero(buf);
                ctx.encrypt(pc, buf)
            }
        }
    }

    pub fn decrypt(&self, pc: u64, buf: &mut [u8]) -> Option<usize> {
        match self {
            Self::Plain(ctx) => ctx.decrypt(pc, buf),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt(pc, buf),
        }
    }

    pub fn decrypt_len(&self, pc: u64, len: [u8; 4]) -> Option<usize> {
        match self {
            Self::Plain(ctx) => ctx.decrypt_len(pc, len),
            Self::Chacha20Poly1305(ctx) => ctx.decrypt_len(pc, len),
        }
    }
}
