use super::super::state::poll_send;
use super::super::ConnectionConfig;
use super::super::{MsgChannelOpen, MsgChannelOpenConfirmation, MsgChannelOpenFailure};
use super::*;

use crate::transport::GenericTransport;

use std::task::{ready, Context, Poll};

#[derive(Debug)]
pub(crate) struct ChannelList {
    config: Arc<ConnectionConfig>,
    slots: Vec<Slot>,
    failures: Vec<u32>, // FIXME poll
}

#[derive(Debug)]
enum Slot {
    Free,
    Open(ChannelHandleInner),
    OpeningInbound1(Box<OpeningInbound1>),
    OpeningInbound2(Box<OpeningInbound2>),
    OpeningOutbound(Box<OpeningOutbound>),
}

#[derive(Debug)]
struct OpeningOutbound {
    pub name: &'static str,
    pub data: Vec<u8>,
    pub sent: bool,
    pub tx: OpenOutboundTx,
}

#[derive(Debug)]
struct OpeningInbound1 {
    pub rid: u32,
    pub rws: u32,
    pub rps: u32,
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
struct OpeningInbound2 {
    pub rid: u32,
    pub ch: ChannelHandleInner,
    pub rx: OpenInboundRx,
}

impl ChannelList {
    pub fn new(config: &Arc<ConnectionConfig>) -> Self {
        Self {
            config: config.clone(),
            slots: Vec::with_capacity(1),
            failures: Vec::with_capacity(0),
        }
    }

    pub fn open_inbound(&mut self, msg: MsgChannelOpen) {
        if let Some(id) = self.alloc() {
            if let Some(slot) = self.slots.get_mut(id) {
                let opening = OpeningInbound1 {
                    rid: msg.sender_channel,
                    rws: msg.initial_window_size,
                    rps: msg.maximum_packet_size,
                    name: msg.name,
                    data: msg.data,
                };
                *slot = Slot::OpeningInbound1(Box::new(opening));
            }
        } else {
            self.failures.push(msg.sender_channel);
        }
    }

    pub fn open_outbound(&mut self, name: &'static str, data: Vec<u8>) -> OpenOutboundRx {
        let (tx, rx) = oneshot::channel();
        let opening = OpeningOutbound {
            name,
            data,
            sent: false,
            tx,
        };
        let opening = Slot::OpeningOutbound(Box::new(opening));
        if let Some(id) = self.alloc() {
            if let Some(slot) = self.slots.get_mut(id) {
                *slot = opening;
                return rx;
            }
        }
        self.slots.push(opening);
        return rx;
    }

    pub fn get_open(&mut self, id: u32) -> Result<&mut ChannelHandleInner, ConnectionError> {
        if let Some(Slot::Open(channel)) = self.slots.get_mut(id as usize) {
            return Ok(channel);
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn accept(&mut self, id: u32, channel: ChannelHandleInner) -> Result<(), ConnectionError> {
        if let Some(slot) = self.slots.get_mut(id as usize) {
            let handle = channel.handle();
            if let Slot::OpeningOutbound(x) = std::mem::replace(slot, Slot::Open(channel)) {
                if x.sent {
                    x.tx.send(Ok(Ok(handle)));
                    return Ok(());
                }
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn reject(&mut self, id: u32, reason: ChannelOpenFailure) -> Result<(), ConnectionError> {
        if let Some(slot) = self.slots.get_mut(id as usize) {
            if let Slot::OpeningOutbound(x) = std::mem::replace(slot, Slot::Free) {
                if x.sent {
                    x.tx.send(Ok(Err(reason)));
                    return Ok(());
                }
            }
        }
        Err(ConnectionError::ChannelIdInvalid)
    }

    pub fn take_open_request(&mut self) -> Option<ChannelOpenRequest> {
        for (id, slot) in self.slots.iter_mut().enumerate() {
            if let Slot::OpeningInbound1(_) = slot {
                if let Slot::OpeningInbound1(x) = std::mem::replace(slot, Slot::Free) {
                    let lid = id as u32;
                    let lws = self.config.channel_max_buffer_size;
                    let lps = self.config.channel_max_packet_size;
                    let rid = x.rid;
                    let rws = x.rws;
                    let rps = x.rps;
                    let ch = ChannelHandleInner::new(lid, lws, lps, rid, rws, rps, false);
                    let handle = ch.handle();
                    let (req, rx) = ChannelOpenRequest::new(x.name, x.data, handle);
                    let y = OpeningInbound2 { rid, rx, ch };
                    *slot = Slot::OpeningInbound2(Box::new(y));
                    return Some(req);
                }
            }
        }
        None
    }

    pub fn poll(
        &mut self,
        cx: &mut Context,
        transport: &mut GenericTransport,
    ) -> Poll<Result<(), ConnectionError>> {
        // Iterate over all channel slots and poll each present channel.
        // Remove channel if the futures is ready (close has been sent _and_ received).
        for (id, slot) in self.slots.iter_mut().enumerate() {
            'inner: loop {
                match slot {
                    Slot::Free => (),
                    Slot::OpeningInbound1(_) => (),
                    Slot::OpeningInbound2(x) => {
                        let e = Err(ChannelOpenFailure::ADMINISTRATIVELY_PROHIBITED);
                        match x.rx.peek(cx).map(|x| x.unwrap_or(e)) {
                            Poll::Ready(Ok(())) => {
                                let msg = MsgChannelOpenConfirmation {
                                    recipient_channel: x.rid,
                                    sender_channel: id as u32,
                                    initial_window_size: self.config.channel_max_buffer_size,
                                    maximum_packet_size: self.config.channel_max_packet_size,
                                    specific: &[],
                                };
                                ready!(poll_send(transport, cx, &msg))?;
                                let y = std::mem::replace(slot, Slot::Free);
                                if let Slot::OpeningInbound2(y) = y {
                                    *slot = Slot::Open(y.ch);
                                }
                                continue 'inner;
                            }
                            Poll::Ready(Err(reason)) => {
                                let msg = MsgChannelOpenFailure::new(x.rid, reason);
                                ready!(poll_send(transport, cx, &msg))?;
                                *slot = Slot::Free;
                            }
                            Poll::Pending => (),
                        }
                    }
                    Slot::OpeningOutbound(x) => {
                        if !x.sent {
                            let msg = MsgChannelOpen::new(
                                x.name,
                                id as u32,
                                self.config.channel_max_buffer_size as u32,
                                self.config.channel_max_packet_size as u32,
                                x.data.clone(),
                            );
                            ready!(poll_send(transport, cx, &msg))?;
                            x.sent = true;
                        }
                    }
                    Slot::Open(channel) => match channel.poll(cx, transport) {
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok(())) => {
                            log::warn!("FREE CHANNEL {}", id);
                            *slot = Slot::Free
                        }
                        Poll::Pending => (),
                    },
                }
                break 'inner;
            }
        }
        log::warn!("OPEN {} of {}", self.open_count(), self.slots.len());
        Poll::Pending
    }

    pub fn queued(&self) -> usize {
        self.failures.len()
    }

    fn alloc(&mut self) -> Option<usize> {
        for (id, slot) in self.slots.iter_mut().enumerate() {
            if let Slot::Free = &slot {
                return Some(id);
            }
        }
        if self.slots.len() < self.config.channel_max_count as usize {
            let id = self.slots.len();
            self.slots.push(Slot::Free);
            return Some(id);
        }
        None
    }

    pub fn open_count(&self) -> usize {
        let mut n = 0;
        for slot in &self.slots {
            if let Slot::Free = slot {
                //
            } else {
                n += 1;
            }
        }
        n
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        for slot in self.slots.iter_mut() {
            match std::mem::replace(slot, Slot::Free) {
                Slot::Free => (),
                Slot::Open(mut x) => x.terminate(e.clone()),
                Slot::OpeningInbound1(_) => (),
                Slot::OpeningInbound2(mut x) => x.ch.terminate(e.clone()),
                Slot::OpeningOutbound(x) => x.tx.send(Err(e.clone())),
            }
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

}
*/
