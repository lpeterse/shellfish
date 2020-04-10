use crate::util::oneshot;

#[derive(Debug)]
pub struct GlobalRequest {
    pub(crate) name: String,
    pub(crate) data: Vec<u8>,
    pub(crate) reply: Option<oneshot::Sender<GlobalReply>>,
}

#[derive(Debug)]
pub enum GlobalReply {
    Success(Vec<u8>),
    Failure,
}

impl GlobalRequest {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn accept(self, data: Vec<u8>) {
        let mut self_ = self;
        if let Some(reply) = self_.reply.take() {
            reply.send(GlobalReply::Success(data))
        }
    }
}
