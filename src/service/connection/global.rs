use super::*;

pub struct GlobalRequest {
    name: String,
    data: Vec<u8>,
    reply: Option<oneshot::Sender<GlobalRequestReply>>,
}

pub enum GlobalRequestReply {
    Success,
    Failure,
}

impl GlobalRequest {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }
    pub fn accept(self) {
        let mut self_ = self;
        if let Some(x) = self_.reply.take() {
            x.send(GlobalRequestReply::Success)
        }
    }
}

impl Drop for GlobalRequest {
    fn drop(&mut self) {
        if let Some(x) = self.reply.take() {
            x.send(GlobalRequestReply::Failure)
        }
    }
}