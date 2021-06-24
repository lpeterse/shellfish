use super::DisconnectReason;
use super::SessionId;
use super::Transport;
use super::TransportError;
use crate::ready;
use std::collections::VecDeque;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct TestTransport {
    tx: mpsc::UnboundedSender<Vec<u8>>,
    tx_buf: Option<Vec<u8>>,
    tx_queue: VecDeque<Vec<u8>>,
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    rx_head: Option<Vec<u8>>,
}

impl TestTransport {
    pub fn new() -> (Self, Self) {
        let (s1, r1) = mpsc::unbounded_channel();
        let (s2, r2) = mpsc::unbounded_channel();
        let self1 = Self {
            tx: s1,
            tx_buf: None,
            tx_queue: VecDeque::new(),
            rx: r2,
            rx_head: None,
        };
        let self2 = Self {
            tx: s2,
            tx_buf: None,
            tx_queue: VecDeque::new(),
            rx: r1,
            rx_head: None,
        };
        (self1, self2)
    }
}

impl Transport for TestTransport {
    fn rx_peek(&mut self, cx: &mut Context) -> Poll<Result<Option<&[u8]>, TransportError>> {
        if self.rx_head.is_none() {
            let e = TransportError::DisconnectByPeer(DisconnectReason::BY_APPLICATION);
            self.rx_head = match self.rx.poll_recv(cx) {
                Poll::Pending => return Poll::Ready(Ok(None)),
                Poll::Ready(Some(x)) => Some(x),
                Poll::Ready(None) => return Poll::Ready(Err(e))
            }
        }
        Poll::Ready(Ok(self.rx_head.as_ref().map(|x| x.as_slice())))
    }

    fn rx_consume(&mut self) -> Result<(), TransportError> {
        assert!(self.rx_head.is_some());
        self.rx_head = None;
        Ok(())
    }

    fn tx_alloc(
        &mut self,
        cx: &mut Context,
        len: usize,
    ) -> Poll<Result<&mut [u8], TransportError>> {
        assert!(self.tx_buf.is_none());
        let mut v = Vec::with_capacity(len);
        v.resize(len, 0);
        self.tx_buf = Some(v);
        Poll::Ready(Ok(self.tx_buf.as_deref_mut().unwrap()))
    }

    fn tx_commit(&mut self) -> Result<(), TransportError> {
        let abc = &self.tx_buf;
        assert!(self.tx_buf.is_some());
        self.tx_queue.push_back(self.tx_buf.take().unwrap());
        Ok(())
    }

    fn tx_flush(&mut self, cx: &mut Context) -> Poll<Result<(), TransportError>> {
        while let Some(v) = self.tx_queue.pop_front() {
            let e = TransportError::DisconnectByPeer(DisconnectReason::BY_APPLICATION);
            self.tx.send(v).map_err(|_| e)?;
        }
        Poll::Ready(Ok(()))
    }

    fn tx_disconnect(
        &mut self,
        _cx: &mut Context,
        _reason: DisconnectReason,
    ) -> Poll<Result<(), TransportError>> {
        Poll::Pending
    }

    fn session_id(&self) -> Result<&SessionId, TransportError> {
        unimplemented!()
    }
}
