use super::*;
use crate::transport::Message;
use crate::util::codec::*;

#[derive(Clone, Debug)]
pub struct MsgSignRequest<'a> {
    pub id: &'a Identity,
    pub data: &'a [u8],
    pub flags: u32,
}

impl<'a> Message for MsgSignRequest<'a> {
    const NUMBER: u8 = 13;
}

impl<'a> Encode for MsgSignRequest<'a> {
    fn size(&self) -> usize {
        std::mem::size_of::<u8>()
            + Encode::size(self.id)
            + std::mem::size_of::<u32>()
            + self.data.len()
            + std::mem::size_of::<u32>()
    }
    fn encode<E: SshEncoder>(&self, e: &mut E) -> Option<()> {
        e.push_u8(<Self as Message>::NUMBER as u8)?;
        e.push(self.id)?;
        e.push_bytes_framed(self.data)?;
        e.push_u32be(self.flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_01() {
        let data: &'static str = "data";
        let id = Identity::from(vec![1, 2, 3]);
        let msg: MsgSignRequest = MsgSignRequest {
            id: &id,
            data: data.as_ref(),
            flags: 123,
        };
        assert_eq!(
            vec![13, 0, 0, 0, 3, 1, 2, 3, 0, 0, 0, 4, 100, 97, 116, 97, 0, 0, 0, 123],
            SliceEncoder::encode(&msg)
        );
    }
}
