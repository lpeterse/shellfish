use super::super::state::poll_send;
use super::super::ConnectionConfig;
use super::super::{MsgChannelOpen, MsgChannelOpenConfirmation, MsgChannelOpenFailure};
use super::state::ChannelState;
use super::*;
use std::sync::Mutex;
use tokio::sync::oneshot;

use crate::transport::GenericTransport;

use std::task::{ready, Context, Poll};

#[derive(Debug)]
pub(crate) struct ChannelList {
    config: Arc<ConnectionConfig>,
    slots2: Vec<Option<Arc<Mutex<ChannelState>>>>,
    failures: Vec<u32>, // FIXME poll
}

impl ChannelList {
    /*
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
    }*/

    pub fn poll(
        &mut self,
        cx: &mut Context,
        transport: &mut GenericTransport,
    ) -> Poll<Result<(), ConnectionError>> {
        Poll::Pending
    }

    /*
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
        // FIXME
        //log::warn!("OPEN {} of {}", self.open_count(), self.slots.len());
        Poll::Pending
    }

    pub fn queued(&self) -> usize {
        self.failures.len()
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
    */

    /*
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
    */
}
