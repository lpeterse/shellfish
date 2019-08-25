use crate::codec::*;
use crate::codec_ssh::*;

#[derive(Clone, Debug)]
pub struct PublicKey {
    algo: String,
    name: Vec<u8>,
}

impl <'a> SshCode<'a> for PublicKey {
    fn size(&self) -> usize {
        panic!("")
    }
    fn encode(&self,c: &mut Encoder<'a>) {
        panic!("")
    }
    fn decode(ctx: &mut Decoder<'a>) -> Option<Self> {
        Some(Self {
            algo: String::from(""), //ctx.take()?,
            name: Vec::from(ctx.take_all()?)
        })
    }
}
