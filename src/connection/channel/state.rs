use super::super::error::ConnectionError;
use super::super::msg::*;
use super::handle::ChannelHandle;
use super::open_failure::ChannelOpenFailure;

use crate::transport::GenericTransport;
use crate::util::buffer::Buffer;
use crate::util::check;
use crate::ready;

use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use tokio::sync::oneshot;

pub const EV_FLUSHED: u8 = 1;
pub const EV_READABLE: u8 = 2;
pub const EV_WRITABLE: u8 = 4;
pub const EV_EOF_SENT: u8 = 8;
pub const EV_EOF_RCVD: u8 = 16;
pub const EV_CLOSE_RCVD: u8 = 32;

#[derive(Debug)]
pub struct ChannelState {
    pub mbs: usize,

    pub lid: u32,
    pub lws: u32,
    pub lmps: u32,

    pub rid: u32,
    pub rws: u32,
    pub rmps: u32,

    pub eof: bool,
    pub eof_sent: bool,
    pub eof_rcvd: bool,

    pub close: bool,
    pub close_sent: bool,
    pub close_rcvd: bool,

    pub stdin: Buffer,
    pub stdout: Buffer,

    pub inner_task_waker: Option<Waker>,
    pub outer_task_waker: Option<Waker>,
    pub outer_task_flags: u8,

    pub resp: Option<oneshot::Sender<Result<ChannelHandle, ChannelOpenFailure>>>,
    pub error: Option<ConnectionError>,
}

impl ChannelState {
    // FIXME
    pub fn new_outbound(
        lid: u32,
        lws: u32,
        lps: u32,
        ext: bool,
        resp: oneshot::Sender<Result<ChannelHandle, ChannelOpenFailure>>,
    ) -> Self {
        Self {
            mbs: lws as usize,

            lid,
            lws,
            lmps: lps,

            rid: 0,
            rws: 0,
            rmps: 0,

            eof: false,
            eof_sent: false,
            eof_rcvd: false,

            close: false,
            close_sent: false,
            close_rcvd: false,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0),

            inner_task_waker: None,
            outer_task_waker: None,
            outer_task_flags: 0,

            resp: Some(resp),
            error: None,
        }
    }

    pub fn new_inbound(
        lid: u32,
        mbs: u32,
        lmps: u32,
        rid: u32,
        rws: u32,
        rmps: u32,
        ext: bool,
        _resp: tokio::sync::oneshot::Receiver<Result<(), ChannelOpenFailure>>,
    ) -> Self {
        Self {
            mbs: mbs as usize,

            lid,
            lws: mbs,
            lmps,

            rid,
            rws,
            rmps,

            eof: false,
            eof_sent: false,
            eof_rcvd: false,

            close: false,
            close_sent: false,
            close_rcvd: false,

            stdin: Buffer::new(0),
            stdout: Buffer::new(0),

            inner_task_waker: None,
            outer_task_waker: None,
            outer_task_flags: 0,

            resp: None,
            error: None,
        }
    }

    pub fn push_open_confirmation(
        &mut self,
        rid: u32,
        rws: u32,
        rps: u32,
        handle: ChannelHandle,
    ) -> Result<(), ConnectionError> {
        // FIXME: Mark channel as open
        self.rid = rid;
        self.rws = rws;
        self.rmps = rps;
        self.resp
            .take()
            .ok_or(ConnectionError::ChannelOpenConfirmationUnexpected)?
            .send(Ok(handle))
            .or(Ok(()))
    }

    pub fn push_open_failure(&mut self, reason: ChannelOpenFailure) -> Result<(), ConnectionError> {
        // FIXME: Mark channel as closed
        self.resp
            .take()
            .ok_or(ConnectionError::ChannelOpenFailureUnexpected)?
            .send(Err(reason))
            .or(Ok(()))
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let len = data.len() as u32;
        check(!self.eof && !self.close).ok_or(ConnectionError::ChannelDataUnexpected)?;
        check(len <= self.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
        check(len <= self.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
        self.lws -= len;
        self.stdin.write_all(data);
        Ok(())
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        /*
        match self.ext {
            Some(ref mut ext) if code == SSH_EXTENDED_DATA_STDERR && !self.eof && !self.close => {
                let len = data.len() as u32;
                check(len <= self.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
                check(len <= self.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
                self.lws -= len;
                ext.rx.write_all(data);
                Ok(())
            }
            _ => Err(ConnectionError::ChannelExtendedDataUnexpected),
        }*/
        panic!()
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        check(!self.close).ok_or(ConnectionError::ChannelWindowAdjustUnexpected)?;
        if (n as u64 + self.rws as u64) > (u32::MAX as u64) {
            return Err(ConnectionError::ChannelWindowAdjustOverflow);
        }
        self.rws += n;
        Ok(())
    }

    pub fn push_request(
        &mut self,
        _name: &str,
        _data: &[u8],
        _want_reply: bool,
    ) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_success(&mut self) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_failure(&mut self) -> Result<(), ConnectionError> {
        panic!()
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        check(!self.eof && !self.close).ok_or(ConnectionError::ChannelEofUnexpected)?;
        self.eof_rcvd = true;
        Ok(())
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        check(!self.close).ok_or(ConnectionError::ChannelCloseUnexpected)?;
        self.close = true;
        self.close_rcvd = true;
        Ok(())
    }

    pub fn poll_with_transport(
        &mut self,
        cx: &mut Context,
        t: &mut GenericTransport,
    ) -> Poll<Result<(), ConnectionError>> {
        // Absent waker means the this is the first call or the channel caused the wakeup.
        if self.inner_task_waker.is_none() {
            // Send data as long as data is available or the remote window size
            while !self.stdout.is_empty() {
                let len = std::cmp::min(self.rmps as usize, self.stdout.len());
                let dat = &self.stdout.as_ref()[..len];
                let msg = MsgChannelData::new(self.rid, dat);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("#{}: Sent MSG_CHANNEL_DATA ({})", self.lid, len);
                self.stdout.consume(len);
            }
            // Send eof if flag set and eof not yet sent.
            if self.eof && !self.eof_sent {
                let msg = MsgChannelEof::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("#{}: Sent MSG_CHANNEL_EOF", self.lid);
                self.eof_sent = true;
            }
            // Send close if flag set and close not yet sent.
            if self.close && !self.close_sent {
                let msg = MsgChannelClose::new(self.rid);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("#{}: Sent MSG_CHANNEL_CLOSE", self.lid);
                self.close_sent = true;
            }
            // Send window adjust message when threshold is reached.
            if let Some(n) = self.recommended_window_adjust() {
                let msg = MsgChannelWindowAdjust::new(self.rid, n);
                ready!(t.poll_send(cx, &msg))?;
                log::debug!("#{}: Sent MSG_CHANNEL_WINDOW_ADJUST ({})", self.lid, n);
                self.lws += n;
            }
        }
        self.inner_task_waker = Some(cx.waker().clone());
        Poll::Ready(Ok(()))
    }

    pub fn recommended_window_adjust(&mut self) -> Option<u32> {
        let threshold = self.mbs / 2;
        if (self.lws as usize) < threshold {
            let buffered = self.stdin.len();
            if buffered < threshold {
                let adjustment = self.mbs - std::cmp::max(self.lws as usize, buffered);
                return Some(adjustment as u32);
            }
        }
        None
    }

    pub fn is_readable(&self) -> bool {
        !self.stdin.is_empty()
    }

    pub fn is_writable(&self) -> bool {
        self.rws > 0 && self.stdout.len() < self.mbs
    }

    pub fn take_outer_waker(&mut self) -> Option<Waker> {
        let test = |b: u8| (self.outer_task_flags & b) != 0;
        check(self.outer_task_waker.is_some())?;
        check(self.outer_task_flags != 0)?;
        if test(EV_FLUSHED) && !self.is_readable() {
            self.outer_task_waker.take()
        } else if test(EV_READABLE) && self.is_readable() {
            self.outer_task_waker.take()
        } else if test(EV_WRITABLE) && self.is_writable() {
            self.outer_task_waker.take()
        } else if test(EV_EOF_SENT) && self.eof_sent {
            self.outer_task_waker.take()
        } else if test(EV_EOF_RCVD) && self.eof_rcvd {
            self.outer_task_waker.take()
        } else if test(EV_CLOSE_RCVD) && self.close_rcvd {
            self.outer_task_waker.take()
        } else {
            None
        }
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::tests::TestTransport;
    use crate::transport::TransportError;
    use std::future::poll_fn;
    use crate::util::runtime::block_on;

    // push_data: Happy path
    #[test]
    fn test_channel_state_push_data_01() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, false);
        assert_eq!(st.push_data(&data), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.outer_task_wake, true);
            assert_eq!(x.std.rx.len(), data.len());
            assert_eq!(x.lws, ws - data.len() as u32);
        })
    }

    // push_data: Unexpected after eof
    #[test]
    fn test_channel_state_push_data_02() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, false);
        assert_eq!(st.push_eof(), Ok(()));
        assert_eq!(
            st.push_data(&data),
            Err(ConnectionError::ChannelDataUnexpected)
        );
    }

    // push_data: Unexpected after close
    #[test]
    fn test_channel_state_push_data_03() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_data(&data),
            Err(ConnectionError::ChannelDataUnexpected)
        );
    }

    // push_data: Window size exceeded
    #[test]
    fn test_channel_state_push_data_04() {
        let ws = 3;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, false);
        assert_eq!(
            st.push_data(&data),
            Err(ConnectionError::ChannelWindowSizeExceeded)
        );
    }

    // push_data: Max packet size exceeded
    #[test]
    fn test_channel_state_push_data_05() {
        let ws = 1024;
        let mps = 3;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, mps, 4, 5, 6, false);
        assert_eq!(
            st.push_data(&data),
            Err(ConnectionError::ChannelMaxPacketSizeExceeded)
        );
    }

    // push_extended_data: Happy path
    #[test]
    fn test_channel_state_push_extended_data_01() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, true);
        assert_eq!(st.push_extended_data(1, &data), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.outer_task_wake, true);
            assert_eq!(x.ext.as_ref().map(|x| x.rx.len()), Some(data.len()));
            assert_eq!(x.lws, ws - data.len() as u32);
        })
    }

    // push_extended_data: Unexpected after eof
    #[test]
    fn test_channel_state_push_extended_data_02() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, true);
        assert_eq!(st.push_eof(), Ok(()));
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelExtendedDataUnexpected)
        );
    }

    // push_extended_data: Unexpected after close
    #[test]
    fn test_channel_state_push_extended_data_03() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, true);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelExtendedDataUnexpected)
        );
    }

    // push_extended_data: Wrong data type code
    #[test]
    fn test_channel_state_push_extended_data_04() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(2, ws, 1024, 4, 5, 6, true);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelExtendedDataUnexpected)
        );
    }

    // push_extended_data: Not an extended data channel
    #[test]
    fn test_channel_state_push_extended_data_05() {
        let ws = 1024;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelExtendedDataUnexpected)
        );
    }

    // push_extended_data: Window size exceeded
    #[test]
    fn test_channel_state_push_extended_data_06() {
        let ws = 3;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, 1024, 4, 5, 6, true);
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelWindowSizeExceeded)
        );
    }

    // push_extended_data: Max packet size exceeded
    #[test]
    fn test_channel_state_push_extended_data_07() {
        let ws = 1024;
        let mps = 3;
        let data = [1, 2, 3, 4];

        let mut st = ChannelHandleInner::new(1, ws, mps, 4, 5, 6, true);
        assert_eq!(
            st.push_extended_data(1, &data),
            Err(ConnectionError::ChannelMaxPacketSizeExceeded)
        );
    }

    // push_eof: Happy path
    #[test]
    fn test_channel_state_push_eof_01() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.push_eof(), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.reof, true);
            assert_eq!(x.rclose, false);
            assert_eq!(x.outer_task_wake, true);
        })
    }

    // push_eof: Unexpected after eof
    #[test]
    fn test_channel_state_push_eof_02() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.push_eof(), Ok(()));
        assert_eq!(st.push_eof(), Err(ConnectionError::ChannelEofUnexpected));
    }

    // push_eof: Unexpected after close
    #[test]
    fn test_channel_state_push_eof_03() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(st.push_eof(), Err(ConnectionError::ChannelEofUnexpected));
    }

    // push_close: Happy path
    #[test]
    fn test_channel_state_push_close_01() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.reof, false);
            assert_eq!(x.rclose, true);
            assert_eq!(x.outer_task_wake, true);
            // Inner task must be woken for eventual cleanup!
            assert_eq!(x.inner_task_wake, true);
        })
    }

    // push_close: Unexpected after close
    #[test]
    fn test_channel_state_push_close_02() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_close(),
            Err(ConnectionError::ChannelCloseUnexpected)
        );
    }

    // push_window_adjust: Happy path
    #[test]
    fn test_channel_state_push_window_adjust_01() {
        let rws = 123;
        let inc = 456;
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, rws, 6, false);
        assert_eq!(st.push_window_adjust(inc), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.rws, rws + inc);
            assert_eq!(x.outer_task_wake, true);
        })
    }

    // push_window_adjust: Ok after eof
    #[test]
    fn test_channel_state_push_window_adjust_02() {
        let rws = 123;
        let inc = 456;
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, rws, 6, false);
        assert_eq!(st.push_eof(), Ok(()));
        assert_eq!(st.push_window_adjust(inc), Ok(()));
        st.with_state(|x| {
            assert_eq!(x.rws, rws + inc);
            assert_eq!(x.outer_task_wake, true);
        })
    }

    // push_window_adjust: Unexpected after close
    #[test]
    fn test_channel_state_push_window_adjust_03() {
        let inc = 123;
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, inc, 6, false);
        assert_eq!(st.push_close(), Ok(()));
        assert_eq!(
            st.push_window_adjust(inc),
            Err(ConnectionError::ChannelWindowAdjustUnexpected)
        );
    }

    // push_window_adjust: Overflow
    #[test]
    fn test_channel_state_push_window_adjust_04() {
        let rws = 1;
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, rws, 6, false);
        assert_eq!(
            st.push_window_adjust(u32::MAX),
            Err(ConnectionError::ChannelWindowAdjustOverflow)
        );
    }

    // poll: Shall register inner task waker and return `Pending`
    #[test]
    fn test_channel_state_poll_01() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let mut t = TestTransport::new();
        block_on(poll_fn(|cx| {
            assert_eq!(st.poll(cx, &mut t), Poll::Pending);
            st.with_state(|x| {
                assert_eq!(x.inner_task_waker.is_some(), true);
            });
            Poll::Ready(())
        }));
    }

    // poll: Shall return `Ready::Ok(())` when close sent and received
    #[test]
    fn test_channel_state_poll_02() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let mut t = TestTransport::new();
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.lclose = true;
                x.lclose_sent = true;
                x.rclose = true;
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Ready(Ok(())));
            Poll::Ready(())
        }));
    }

    // poll: Shall return `Ready::Err(_)` when an error occurs
    #[test]
    fn test_channel_state_poll_03() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let mut t = TestTransport::new();
        let e = TransportError::MessageIntegrity;
        t.set_error(e);
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.lclose = true;
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Ready(Err(e.into())));
            Poll::Ready(())
        }));
    }

    // poll: Shall send data when present
    #[test]
    fn test_channel_state_poll_04() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 3, false);
        let mut t = TestTransport::new();
        t.set_tx_ready(true);
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.std.tx.write_all(&[1, 2, 3, 4, 5, 6, 7, 8]);
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Pending);
            st.with_state(|x| {
                assert_eq!(x.inner_task_wake, false);
                assert_eq!(x.outer_task_wake, true);
            });
            Poll::Ready(())
        }));
        assert_eq!(
            t.tx_buf(),
            vec![
                vec![94, 0, 0, 0, 4, 0, 0, 0, 3, 1, 2, 3],
                vec![94, 0, 0, 0, 4, 0, 0, 0, 2, 4, 5]
            ]
        );
    }

    // poll: Shall send extended data when present
    #[test]
    fn test_channel_state_poll_05() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 3, true);
        let mut t = TestTransport::new();
        t.set_tx_ready(true);
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.ext
                    .as_mut()
                    .map(|ext| ext.tx.write_all(&[1, 2, 3, 4, 5, 6, 7, 8]));
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Pending);
            st.with_state(|x| {
                assert_eq!(x.inner_task_wake, false);
                assert_eq!(x.outer_task_wake, true);
            });
            Poll::Ready(())
        }));
        assert_eq!(
            t.tx_buf(),
            vec![
                vec![95, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 3, 1, 2, 3],
                vec![95, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 2, 4, 5]
            ]
        );
    }

    // poll: Shall send eof when present
    #[test]
    fn test_channel_state_poll_07() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let mut t = TestTransport::new();
        t.set_tx_ready(true);
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.leof = true;
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Pending);
            st.with_state(|x| {
                assert_eq!(x.leof_sent, true);
                assert_eq!(x.lclose_sent, false);
                assert_eq!(x.inner_task_wake, false);
                assert_eq!(x.outer_task_wake, false);
            });
            Poll::Ready(())
        }));
        assert_eq!(t.tx_buf(), vec![vec![96, 0, 0, 0, 4]]);
    }

    // poll: Shall send close when present
    #[test]
    fn test_channel_state_poll_08() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        let mut t = TestTransport::new();
        t.set_tx_ready(true);
        block_on(poll_fn(|cx| {
            st.with_state(|x| {
                x.lclose = true;
                x.inner_task_wake = true;
            });
            assert_eq!(st.poll(cx, &mut t), Poll::Pending);
            st.with_state(|x| {
                assert_eq!(x.leof_sent, false);
                assert_eq!(x.lclose_sent, true);
                assert_eq!(x.inner_task_wake, false);
                assert_eq!(x.outer_task_wake, false);
            });
            Poll::Ready(())
        }));
        assert_eq!(t.tx_buf(), vec![vec![97, 0, 0, 0, 4]]);
    }

    #[test]
    fn test_channel_state_terminate_01() {
        let mut st = ChannelHandleInner::new(1, 2, 3, 4, 5, 6, false);
        st.terminate(ConnectionError::ResourceExhaustion);
        st.with_state(|x| {
            assert_eq!(x.error, Some(ConnectionError::ResourceExhaustion));
            assert_eq!(x.outer_task_wake, true);
        })
    }
}
*/
