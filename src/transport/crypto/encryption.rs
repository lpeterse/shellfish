mod chacha20_poly1305;
mod plain;

use self::chacha20_poly1305::*;
use self::plain::*;
use super::*;
use super::super::*;

pub trait EncryptionAlgorithm {
    const NAME: &'static str;
}

pub struct Chacha20Poly1305AtOpensshDotCom {}

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

pub type EncryptionConfig = CipherConfig;
pub type DecryptionConfig = CipherConfig;

#[derive(Debug)]
pub enum CipherContext {
    Plain(PlainContext),
    Chacha20Poly1305(Chacha20Poly1305Context),
}

impl CipherContext {
    pub fn new() -> Self {
        Self::Plain(PlainContext::new())
    }

    pub fn update(&mut self, mut config: CipherConfig) -> Option<()> {
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
            _ => return None,
        }
        Some(())
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

    pub fn decrypt_len(&self, pc: u64, len: [u8; 4]) -> Option<usize> {
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
            Self::Plain(_) => PlainContext::padding_len(payload_len),
            Self::Chacha20Poly1305(_) => Chacha20Poly1305Context::padding_len(payload_len),
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_01() {
        let ctx = CipherContext::new();
        match ctx {
            CipherContext::Plain(_) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_update_01() {
        let mut ctx = CipherContext::new();
        let config = CipherConfig {
            ea: "chacha20-poly1305@openssh.com",
            ca: "none",
            ma: None,
            ke: KeyStreams::new_sha256(&[], &[], &[]).c(),
        };
        // The first update shall create a new instance of chacha20.
        ctx.update(config.clone()).unwrap();
        match ctx {
            CipherContext::Chacha20Poly1305(_) => (),
            _ => panic!(""),
        }
        // The second update shalll update the existing instance.
        // This is not so much a perfomance optimization, but will ensure that that
        // the old keys will vanish by being overwritten.
        ctx.update(config).unwrap();
        match ctx {
            CipherContext::Chacha20Poly1305(_) => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_update_02() {
        let mut ctx = CipherContext::new();
        let config = CipherConfig {
            ea: "chacha20-poly1305@openssh.com",
            ca: "some",
            ma: None,
            ke: KeyStreams::new_sha256(&[], &[], &[]).c(),
        };
        // This combination shall not work.
        match ctx.update(config) {
            None => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_update_03() {
        let mut ctx = CipherContext::new();
        let config = CipherConfig {
            ea: "chacha20-poly1305@openssh.com",
            ca: "none",
            ma: Some("none"),
            ke: KeyStreams::new_sha256(&[], &[], &[]).c(),
        };
        // This combination shall not work.
        match ctx.update(config) {
            None => (),
            _ => panic!(""),
        }
    }

    #[test]
    fn test_encrypt_plain_01() {
        let ctx = CipherContext::new();
        let buf1 = [0, 0, 0, 12, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut buf2 = buf1.clone();
        ctx.encrypt(23, &mut buf2[..]);
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_encrypt_chacha_01() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let buf1 = [
            227, 107, 184, 85, 22, 165, 167, 251, 182, 172, 219, 51, 204, 70, 149, 248, 19, 33,
            146, 117, 222, 231, 131, 147, 93, 123, 39, 124, 80, 57, 137, 90, 53, 182, 210, 75,
        ];
        let mut buf2 = [
            0, 0, 0, 16, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];
        ctx.encrypt(23, &mut buf2[..]);
        assert_eq!(&buf1[..], &buf2[..]);
    }

    #[test]
    fn test_decrypt_plain_01() {
        let ctx = CipherContext::new();
        let buf1 = [0, 0, 0, 12, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut buf2 = buf1.clone();
        ctx.decrypt(23, &mut buf2[..]);
        assert_eq!(buf1, buf2);
    }

    #[test]
    fn test_decrypt_chacha_01() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let mut buf1 = [
            227, 107, 184, 85, 22, 165, 167, 251, 182, 172, 219, 51, 204, 70, 149, 248, 19, 33,
            146, 117, 222, 231, 131, 147, 93, 123, 39, 124, 80, 57, 137, 90, 53, 182, 210, 75,
        ];
        let buf2 = [
            227, 107, 184, 85, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 222, 231, 131, 147,
            93, 123, 39, 124, 80, 57, 137, 90, 53, 182, 210, 75,
        ];
        ctx.decrypt(23, &mut buf1[..]);
        assert_eq!(&buf1[..], &buf2[..]);
    }

    #[test]
    fn test_decrypt_len_plain_01() {
        let ctx = CipherContext::new();
        let buf = [0, 0, 0, 12];
        assert_eq!(ctx.decrypt_len(23, buf), Some(16));
    }

    #[test]
    fn test_decrypt_len_chacha_01() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let buf = [227, 107, 184, 85];
        assert_eq!(ctx.decrypt_len(23, buf), Some(36));
    }

    #[test]
    fn test_packet_plain_empty() {
        let ctx = CipherContext::new();
        let packet = ctx.packet(&());
        // size = 4 (packet len) + 1 (padding len) + 0 (payload len) + 11 (padding)
        assert_eq!(packet.size(), 16);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[0, 0, 0, 12, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]
        );
    }

    #[test]
    fn test_packet_plain_bytes1() {
        let ctx = CipherContext::new();
        let packet = ctx.packet(&Bytes1 {});
        // size = 4 (packet len) + 1 (padding len) + 1 (payload len) + 10 (padding)
        assert_eq!(packet.size(), 16);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[0, 0, 0, 12, 10, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]
        );
    }

    #[test]
    fn test_packet_plain_bytes8() {
        let ctx = CipherContext::new();
        let packet = ctx.packet(&Bytes8 {});
        // size = 4 (packet len) + 1 (padding len) + 8 (payload len) + 11 (padding)
        // RFC: "There must be at least 4 bytes of padding"
        assert_eq!(packet.size(), 24);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[0, 0, 0, 20, 11, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]
        );
    }

    #[test]
    fn test_packet_plain_bytes16() {
        let ctx = CipherContext::new();
        let packet = ctx.packet(&Bytes16 {});
        assert_eq!(packet.size(), 32);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[
                0, 0, 0, 28, 11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0
            ][..]
        );
    }

    #[test]
    fn test_packet_chacha_empty() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let packet = ctx.packet(&());
        // ChaCha20Poly1305 is authentication with additional data (AAD).
        // The packet without packet len field itself must be a multiple of 8 (this is not exactly
        // obvious when reading the original transport layer RFC).
        assert_eq!(packet.size(), 36);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[
                0, 0, 0, 16, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0
            ][..]
        );
    }

    #[test]
    fn test_packet_chacha_bytes1() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let packet = ctx.packet(&Bytes1 {});
        assert_eq!(packet.size(), 36);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[
                0, 0, 0, 16, 14, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0
            ][..]
        );
    }

    #[test]
    fn test_packet_chacha_bytes8() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let packet = ctx.packet(&Bytes8 {});
        assert_eq!(packet.size(), 36);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[
                0, 0, 0, 16, 7, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0
            ][..]
        );
    }

    #[test]
    fn test_packet_chacha_bytes16() {
        let ks = KeyStreams::new_sha256(&[], &[], &[]);
        let ctx = CipherContext::Chacha20Poly1305(Chacha20Poly1305Context::new(&mut ks.c()));
        let packet = ctx.packet(&Bytes16 {});
        assert_eq!(packet.size(), 44);
        assert_eq!(
            &SliceEncoder::encode(&packet)[..],
            &[
                0, 0, 0, 24, 7, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
            ][..]
        );
    }

    pub struct Bytes1 {}

    impl Encode for Bytes1 {
        fn size(&self) -> usize {
            1
        }
        fn encode<E: Encoder>(&self, e: &mut E) {
            e.push_u8(1);
        }
    }

    pub struct Bytes8 {}

    impl Encode for Bytes8 {
        fn size(&self) -> usize {
            8
        }
        fn encode<E: Encoder>(&self, e: &mut E) {
            e.push_u8(1);
            e.push_u8(2);
            e.push_u8(3);
            e.push_u8(4);
            e.push_u8(5);
            e.push_u8(6);
            e.push_u8(7);
            e.push_u8(8);
        }
    }

    pub struct Bytes16 {}

    impl Encode for Bytes16 {
        fn size(&self) -> usize {
            16
        }
        fn encode<E: Encoder>(&self, e: &mut E) {
            e.push_u8(1);
            e.push_u8(2);
            e.push_u8(3);
            e.push_u8(4);
            e.push_u8(5);
            e.push_u8(6);
            e.push_u8(7);
            e.push_u8(8);
            e.push_u8(9);
            e.push_u8(10);
            e.push_u8(11);
            e.push_u8(12);
            e.push_u8(13);
            e.push_u8(14);
            e.push_u8(15);
            e.push_u8(16);
        }
    }
}

*/
