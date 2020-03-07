use crate::algorithm::auth::*;
use crate::codec::*;
use crate::message::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a> {
    pub identity: &'a Identity,
    pub data: &'a [u8],
    pub flags: u32,
}

impl<'a> Message for MsgSignRequest<'a> {
    const NUMBER: u8 = 13;
}

impl<'a> Encode for MsgSignRequest<'a> {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
            + Encode::size(self.identity)
            + Encode::size(self.data)
            + std::mem::size_of::<u32>()
    }
    fn encode<E: Encoder>(&self, e: &mut E) {
        e.push_u8(<Self as Message>::NUMBER as u8);
        Encode::encode(self.identity, e);
        Encode::encode(self.data, e);
        e.push_u32be(self.flags);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let data: &'static str = "data";
        let identity = Identity::PublicKey(PublicKey::Ed25519(SshEd25519PublicKey([2; 32])));
        let msg: MsgSignRequest = MsgSignRequest {
            identity: &identity,
            data: data.as_ref(),
            flags: 123,
        };
        assert_eq!(
            vec![
                13, 0, 0, 0, 51, 0, 0, 0, 11, 115, 115, 104, 45, 101, 100, 50, 53, 53, 49, 57, 0,
                0, 0, 32, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
                2, 2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 8, 0, 0, 0, 4, 100, 97, 116, 97, 0, 0, 0, 123
            ],
            BEncoder::encode(&msg)
        );
    }
}
