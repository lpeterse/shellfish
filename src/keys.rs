use crate::parser::*;

#[derive(Clone, Debug)]
pub struct PublicKey {
    algo: String,
    name: Vec<u8>,
}

impl <'a> Parser<'a> for PublicKey {
    fn parse(ctx: &mut Context<'a>) -> Option<Self> {
        Some(Self {
            algo: String::from(""), //ctx.take()?,
            name: Vec::from(ctx.take_all()?)
        })
    }
}