use num::BigUint;

use crate::codec::*;
use crate::codec_ssh::*;

#[derive(Clone, Debug)]
pub struct RsaPublicKey {
    pub public_e: BigUint,
    pub public_n: BigUint,
}

impl <'a> SshCodec<'a> for RsaPublicKey {
    fn size(&self) -> usize {
        0
    }
    fn encode(&self,c: &mut Encoder<'a>) {

    }
    fn decode(c: &mut Decoder<'a>) -> Option<Self> {
        let e = SshCodec::decode(c)?;
        let n = SshCodec::decode(c)?;
        Some(RsaPublicKey {
            public_e: e,
            public_n: n,
        })
    }
}

/*
getRsaPublicKey :: Get RSA.PublicKey
getRsaPublicKey = do
    (e,_) <- getIntegerAndSize
    (n,s) <- getIntegerAndSize
    when (s > 8192 `div` 8) (fail "key size not supported")
    pure $ RSA.PublicKey s n e
    where
        -- Observing the encoded length is far cheaper than calculating the
        -- log2 of the resulting integer.
        getIntegerAndSize :: Get (Integer, Int)
        getIntegerAndSize = do
            ws <- dropWhile (== 0) . (BA.unpack :: BS.ByteString -> [Word8]) <$> getString -- remove leading 0 bytes
            pure (foldl' (\acc w8-> acc * 256 + fromIntegral w8) 0 ws, length ws)
*/