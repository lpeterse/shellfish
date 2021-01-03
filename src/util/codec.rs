mod decoder;
mod encoder;
mod ref_decoder;
mod ref_encoder;
mod size_encoder;
mod ssh_decode;
mod ssh_decoder;
mod ssh_encode;
mod ssh_encoder;

pub use self::decoder::*;
pub use self::encoder::*;
pub use self::ref_decoder::*;
pub use self::ref_encoder::*;
pub use self::size_encoder::*;
pub use self::ssh_decode::*;
pub use self::ssh_decoder::*;
pub use self::ssh_encode::*;
pub use self::ssh_encoder::*;

/// Utility type for the encoding of SSH data structures as specified in RFC 4251 and others.
pub struct SshCodec;

impl SshCodec {
    /// Determine the size in bytes of the `SshEncode`d form.
    ///
    /// This only iterates the data structure and does not really encode it nor
    /// does it allocate anything.
    pub fn size<T: SshEncode>(x: &T) -> Option<usize> {
        let mut e = SizeEncoder::new();
        e.push(x)?;
        Some(e.into())
    }

    /// `SshEncode` a given structue into a `Vec<u8>`.
    pub fn encode<T: SshEncode>(x: &T) -> Option<Vec<u8>> {
        let size = Self::size(x)?;
        let mut vec = Vec::with_capacity(size);
        vec.resize(size, 0);
        let mut e = RefEncoder::new(&mut vec);
        e.push(x)?;
        crate::util::check(e.is_full())?;
        Some(vec)
    }

    /// `SshEncode` a given structue into supplied buffer of correct size.
    pub fn encode_into<'a, T: SshEncode>(x: &T, buf: &'a mut [u8]) -> Option<()> {
        let mut e = RefEncoder::new(buf);
        e.push(x)?;
        crate::util::check(e.is_full())
    }

    /// Try to `SshDecode` the given input as `T`.
    ///
    /// All bytes of input must be consumed or the decoding will fail.
    pub fn decode<'a, T: SshDecodeRef<'a>>(buf: &'a [u8]) -> Option<T> {
        let mut d = RefDecoder::new(buf);
        let t = d.take()?;
        d.expect_eoi()?;
        Some(t)
    }
}
