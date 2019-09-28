use crate::codec::*;
use std::ops::Deref;

pub trait ChannelType {
    type Open: Decode;
    type Confirmation: Decode;
    type Request: ChannelRequest + Encode;
    type SpecificState: Default;

    const NAME: &'static str;
}

pub trait ChannelRequest {
    fn name(&self) -> &'static str;
}

impl <T: ChannelRequest> ChannelRequest for &T {
    fn name(&self) -> &'static str {
        self.deref().name()
    }
}
