mod hostkeys;
mod keepalive;

pub use self::hostkeys::*;
pub use self::keepalive::*;

use crate::util::codec::*;
use tokio::sync::oneshot;

pub trait Global: Sized {
    const NAME: &'static str;
    type RequestData: SshEncode + SshDecode + std::fmt::Debug;
}

pub trait GlobalWantReply: Global {
    type ResponseData: SshEncode + SshDecode + std::fmt::Debug;
}

impl Global for () {
    const NAME: &'static str = "";
    type RequestData = Vec<u8>;
}

impl GlobalWantReply for () {
    type ResponseData = Vec<u8>;
}

#[derive(Debug)]
pub struct GlobalRequest<T: Global = ()> {
    name: String,
    data: <T as Global>::RequestData,
}

impl GlobalRequest<()> {
    pub(crate) fn new(name: String, data: Vec<u8>) -> Self {
        Self { name, data }
    }

    pub fn interpret<T: Global>(self) -> Result<GlobalRequest<T>, Self> {
        if !T::NAME.is_empty() && T::NAME == &self.name {
            if let Some(data) = SshCodec::decode(&self.data) {
                return Ok(GlobalRequest {
                    name: self.name,
                    data,
                });
            }
        }
        Err(self)
    }
}

impl<T: Global> GlobalRequest<T> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn data(&self) -> &T::RequestData {
        &self.data
    }
}

#[derive(Debug)]
pub struct GlobalRequestWantReply<T: GlobalWantReply = ()> {
    name: String,
    data: <T as Global>::RequestData,
    repl: oneshot::Sender<Vec<u8>>,
}

impl GlobalRequestWantReply<()> {
    pub(crate) fn new(name: String, data: Vec<u8>, repl: oneshot::Sender<Vec<u8>>) -> Self {
        Self { name, data, repl }
    }

    pub fn interpret<T: GlobalWantReply>(self) -> Result<GlobalRequestWantReply<T>, Self> {
        if !T::NAME.is_empty() && T::NAME == &self.name {
            if let Some(data) = SshCodec::decode(&self.data) {
                return Ok(GlobalRequestWantReply {
                    name: self.name,
                    data,
                    repl: self.repl
                });
            }
        }
        Err(self)
    }
}



impl<T: GlobalWantReply> GlobalRequestWantReply<T> {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn data(&self) -> &T::RequestData {
        &self.data
    }

    pub fn accept(self, data: T::ResponseData) {
        SshCodec::encode(&data)
            .and_then(|data| self.repl.send(data).ok())
            .unwrap_or(())
    }

    pub fn reject(self) {
        drop(self)
    }
}

#[macro_export]
macro_rules! interpret {
    ($request:ident, $ty:ty, $block:block) => {
        #[allow(unused_variables)]
        let $request = match $request.interpret::<$ty>() {
            Err(x) => x,
            Ok($request) => {
                return $block;
            }
        };
    };
}
