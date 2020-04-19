use super::super::{ConnectionConfig, MsgChannelOpen, Terminate};
use super::*;

use std::slice::IterMut;

#[derive(Debug)]
pub(crate) struct ChannelSlots {
    config: Arc<ConnectionConfig>,
    elements: Vec<ChannelSlot>,
    failures: Vec<u32>, // FIXME poll
}

#[derive(Debug)]
pub(crate) enum ChannelSlot {
    Free,
    Open(ChannelHandle),
    OpeningInbound1(Box<OpeningInbound1>),
    OpeningInbound2(Box<OpeningInbound2>),
    OpeningOutbound(Box<OpeningOutbound>),
}

#[derive(Debug)]
pub(crate) struct OpeningOutbound {
    pub name: &'static str,
    pub data: Vec<u8>,
    pub sent: bool,
    pub tx: OpenOutboundTx,
}

#[derive(Debug)]
pub(crate) struct OpeningInbound1 {
    pub rid: u32,
    pub rws: u32,
    pub rps: u32,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct OpeningInbound2 {
    pub rid: u32,
    pub ch: ChannelHandle,
    pub rx: OpenInboundRx,
}

impl ChannelSlots {
    pub fn new(config: &Arc<ConnectionConfig>) -> Self {
        Self {
            config: config.clone(),
            elements: Vec::with_capacity(1),
            failures: Vec::with_capacity(0),
        }
    }

    pub fn open_inbound(&mut self, msg: MsgChannelOpen) {
        if let Some(id) = self.alloc() {
            if let Some(slot) = self.elements.get_mut(id) {
                let opening = OpeningInbound1 {
                    rid: msg.sender_channel,
                    rws: msg.initial_window_size,
                    rps: msg.maximum_packet_size,
                    name: msg.name,
                    data: msg.data,
                };
                *slot = ChannelSlot::OpeningInbound1(Box::new(opening));
            }
        } else {
            self.failures.push(msg.sender_channel);
        }
    }

    pub fn open_outbound(&mut self, name: &'static str, data: Vec<u8>) -> Option<OpenOutboundRx> {
        let id = self.alloc()?;
        let slot = self.elements.get_mut(id)?;
        let (tx, rx) = oneshot::channel();
        let opening = OpeningOutbound {
            name,
            data,
            sent: false,
            tx,
        };
        *slot = ChannelSlot::OpeningOutbound(Box::new(opening));
        Some(rx)
    }

    pub fn get_open(&mut self, id: u32) -> Result<&mut ChannelHandle, ConnectionError> {
        if let Some(ChannelSlot::Open(channel)) = self.elements.get_mut(id as usize) {
            return Ok(channel);
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn accept(&mut self, id: u32, channel: ChannelHandle) -> Result<(), ConnectionError> {
        if let Some(slot) = self.elements.get_mut(id as usize) {
            if let ChannelSlot::OpeningOutbound(x) =
                std::mem::replace(slot, ChannelSlot::Open(channel.clone()))
            {
                if x.sent {
                    x.tx.send(Ok(Ok(channel)));
                    return Ok(());
                }
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn reject(
        &mut self,
        id: u32,
        reason: ChannelOpenFailureReason,
    ) -> Result<(), ConnectionError> {
        if let Some(slot) = self.elements.get_mut(id as usize) {
            if let ChannelSlot::OpeningOutbound(x) = std::mem::replace(slot, ChannelSlot::Free) {
                if x.sent {
                    x.tx.send(Ok(Err(reason)));
                    return Ok(());
                }
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, ChannelSlot> {
        self.elements.iter_mut()
    }

    pub fn take_open_request(&mut self) -> Option<ChannelOpenRequest> {
        for (id, slot) in self.elements.iter_mut().enumerate() {
            if let ChannelSlot::OpeningInbound1(_) = slot {
                if let ChannelSlot::OpeningInbound1(x) = std::mem::replace(slot, ChannelSlot::Free)
                {
                    let lid = id as u32;
                    let lws = self.config.channel_max_window_size;
                    let lps = self.config.channel_max_packet_size;
                    let rid = x.rid;
                    let rws = x.rws;
                    let rps = x.rps;
                    let ch = ChannelHandle::new(lid, lws, lps, rid, rws, rps);
                    let (req, rx) = ChannelOpenRequest::new(x.name, x.data, ch.clone());
                    let y = OpeningInbound2 { rid, rx, ch };
                    std::mem::replace(slot, ChannelSlot::OpeningInbound2(Box::new(y)));
                    return Some(req);
                }
            }
        }
        None
    }

    fn alloc(&mut self) -> Option<usize> {
        for (id, slot) in self.elements.iter_mut().enumerate() {
            if let ChannelSlot::Free = &slot {
                return Some(id);
            }
        }
        if self.elements.len() < self.config.channel_max_count as usize {
            let id = self.elements.len();
            self.elements.push(ChannelSlot::Free);
            return Some(id);
        }
        None
    }
}

impl Terminate for ChannelSlots {
    fn terminate(&mut self, e: ConnectionError) {
        for slot in self.iter_mut() {
            match std::mem::replace(slot, ChannelSlot::Free) {
                ChannelSlot::Free => (),
                ChannelSlot::Open(mut x) => x.terminate(e),
                ChannelSlot::OpeningInbound1(_) => (),
                ChannelSlot::OpeningInbound2(mut x) => x.ch.terminate(e),
                ChannelSlot::OpeningOutbound(x) => x.tx.send(Err(e)),
            }
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_01() {
        let m = ChannelSlots::<()>::new(23);
        assert_eq!(m.capacity, 23);
        assert_eq!(m.elements.len(), 0);
        assert_eq!(m.elements.capacity(), 1);
    }

    #[test]
    fn test_len_01() {
        let m = ChannelSlots::<()>::new(23);
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn test_alloc_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.len(), 1);
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.alloc(), None);
    }

    #[test]
    fn test_get_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.get(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_insert_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.insert(0, ()), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_remove_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_remove_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.remove(0), Ok(()));
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
    }

    #[test]
    fn test_alloc_insert_get_01() {
        let mut m = ChannelSlots::<()>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, ()), Ok(()));
        assert_eq!(m.get(0), Ok(&mut ()));
    }

    #[test]
    fn test_alloc_insert_get_02() {
        let mut m = ChannelSlots::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, 23), Ok(()));
        assert_eq!(m.get(0), Ok(&mut 23));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
    }

    #[test]
    fn test_alloc_insert_get_remove_01() {
        let mut m = ChannelSlots::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.insert(0, 23), Ok(()));
        assert_eq!(m.get(0), Ok(&mut 23));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
        assert_eq!(m.remove(0), Ok(()));
        assert_eq!(m.remove(0), Err(ConnectionError::ChannelIdInvalid));
        assert_eq!(m.get(0), Err(ConnectionError::ChannelIdInvalid));
        assert_eq!(m.get(1), Ok(&mut 47));
        assert_eq!(m.alloc(), Some(0));
    }

    #[test]
    fn test_iter_01() {
        let mut m = ChannelSlots::<usize>::new(2);
        assert_eq!(m.alloc(), Some(0));
        assert_eq!(m.alloc(), Some(1));
        assert_eq!(m.insert(1, 47), Ok(()));
        assert_eq!(m.get(1), Ok(&mut 47));
        let mut n: usize = 0;
        for i in m.iter() {
            n += 1;
            assert_eq!(i, &mut 47);
        }
        assert_eq!(n, 1);
    }
}
*/
