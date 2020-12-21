use super::super::MsgChannelClose;
use super::super::MsgChannelData;
use super::super::MsgChannelEof;
use super::super::MsgChannelExtendedData;
use super::super::MsgChannelWindowAdjust;
use super::handle::*;
use super::state::*;
use super::*;

use crate::transport::Transport;
use crate::transport::TransportExt;
use crate::util::check;

use async_std::task::{ready, Context};
use std::sync::{Arc, Mutex};

const SSH_EXTENDED_DATA_STDERR: u32 = 1;

#[derive(Debug, Clone)]
pub struct ChannelHandleInner(Arc<Mutex<ChannelState>>);

impl ChannelHandleInner {
    pub fn new(lid: u32, lmws: u32, lmps: u32, rid: u32, rws: u32, rmps: u32, ext: bool) -> Self {
        let x = ChannelState::new(lid, lmws, lmps, rid, rws, rmps, ext);
        Self(Arc::new(Mutex::new(x)))
    }

    pub fn handle(&self) -> ChannelHandle {
        (&self.0).into()
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        self.with_state(|x| {
            let len = data.len() as u32;
            check(!x.reof && !x.rclose).ok_or(ConnectionError::ChannelDataUnexpected)?;
            check(len <= x.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
            check(len <= x.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
            x.lws -= len;
            x.std.rx.write_all(data);
            x.outer_task_wake = true;
            Ok(())
        })
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        self.with_state(|x| match x.ext {
            Some(ref mut ext) if code == SSH_EXTENDED_DATA_STDERR && !x.reof && !x.rclose => {
                let len = data.len() as u32;
                check(len <= x.lws).ok_or(ConnectionError::ChannelWindowSizeExceeded)?;
                check(len <= x.lmps).ok_or(ConnectionError::ChannelMaxPacketSizeExceeded)?;
                x.lws -= len;
                ext.rx.write_all(data);
                x.outer_task_wake = true;
                Ok(())
            }
            _ => Err(ConnectionError::ChannelExtendedDataUnexpected),
        })
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        self.with_state(|x| {
            check(!x.reof && !x.rclose).ok_or(ConnectionError::ChannelEofUnexpected)?;
            x.reof = true;
            x.outer_task_wake = true;
            Ok(())
        })
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        self.with_state(|x| {
            check(!x.rclose).ok_or(ConnectionError::ChannelCloseUnexpected)?;
            x.rclose = true;
            x.outer_task_wake = true;
            // Inner task must be woken for eventual cleanup!
            x.inner_task_wake = true;
            Ok(())
        })
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        self.with_state(|x| {
            check(!x.rclose).ok_or(ConnectionError::ChannelWindowAdjustUnexpected)?;
            if (n as u64 + x.rws as u64) > (u32::MAX as u64) {
                return Err(ConnectionError::ChannelWindowAdjustOverflow);
            }
            x.rws += n;
            x.outer_task_wake = true;
            Ok(())
        })
    }

    pub fn push_request(&mut self, _request: Vec<u8>) -> Result<(), ConnectionError> {
        todo!("push_request")
    }

    pub fn push_success(&mut self) -> Result<(), ConnectionError> {
        todo!("push_success")
    }

    pub fn push_failure(&mut self) -> Result<(), ConnectionError> {
        todo!("push_failure")
    }

    pub fn poll(
        &mut self,
        cx: &mut Context,
        t: &mut Box<dyn Transport>,
    ) -> Poll<Result<(), ConnectionError>> {
        self.with_state(|x| {
            ready!(x.poll_inner_task_woken(cx));
            if !x.lclose_sent {
                while !x.std.tx.is_empty() {
                    let len = std::cmp::min(x.rmps, x.std.tx.len() as u32);
                    let len = std::cmp::min(x.rws, len);
                    if len > 0 {
                        let data = &x.std.tx.as_ref()[..len as usize];
                        let msg = MsgChannelData::new(x.rid, data);
                        ready!(TransportExt::poll_send(t, cx, &msg))?;
                        log::debug!("Channel {}: Sent MSG_CHANNEL_DATA ({})", x.lid, len);
                        x.rws -= len;
                        x.std.tx.consume(len as usize);
                        x.outer_task_wake = true;
                    } else {
                        break;
                    }
                }
                if let Some(ref mut ext) = x.ext {
                    let code = SSH_EXTENDED_DATA_STDERR;
                    while !ext.tx.is_empty() {
                        let len = std::cmp::min(x.rmps, ext.tx.len() as u32);
                        let len = std::cmp::min(x.rws, len);
                        if len > 0 {
                            let data = &ext.tx.as_ref()[..len as usize];
                            let msg = MsgChannelExtendedData::new(x.rid, code, data);
                            ready!(TransportExt::poll_send(t, cx, &msg))?;
                            log::debug!(
                                "Channel {}: Sent MSG_CHANNEL_EXTENDED_DATA ({})",
                                x.lid,
                                len
                            );
                            x.rws -= len;
                            ext.tx.consume(len as usize);
                            x.outer_task_wake = true;
                        } else {
                            break;
                        }
                    }
                }
                if let Some(n) = x.local_window_adjust() {
                    let msg = MsgChannelWindowAdjust::new(x.rid, n);
                    ready!(TransportExt::poll_send(t, cx, &msg))?;
                    log::debug!("Channel {}: Sent MSG_CHANNEL_WINDOW_ADJUST ({})", x.lid, n);
                    x.lws += n;
                }
                if x.leof && !x.leof_sent {
                    let msg = MsgChannelEof::new(x.rid);
                    ready!(TransportExt::poll_send(t, cx, &msg))?;
                    log::debug!("Channel {}: Sent MSG_CHANNEL_EOF", x.lid);
                    x.leof_sent = true;
                }
                if x.lclose {
                    let msg = MsgChannelClose::new(x.rid);
                    ready!(TransportExt::poll_send(t, cx, &msg))?;
                    log::debug!("Channel {}: Sent MSG_CHANNEL_CLOSE", x.lid);
                    x.lclose_sent = true;
                }
            }
            if x.rclose && x.lclose_sent {
                Poll::Ready(Ok(()))
            } else {
                // Assure that polling it the next time is a noop unless there's something todo.
                x.inner_task_wake = false;
                Poll::Pending
            }
        })
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        self.with_state(|x| {
            x.error = Some(e);
            x.outer_task_wake = true;
        })
    }

    pub fn with_state<F, X>(&self, f: F) -> X
    where
        F: FnOnce(&mut ChannelState) -> X,
    {
        let (result, waker) = {
            let mut state = self.0.lock().unwrap();
            (f(&mut state), state.outer_task_waker())
        };
        if let Some(waker) = waker {
            waker.wake()
        }
        result
    }
}

/*

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::tests::TestTransport;
    use crate::transport::TransportError;
    use async_std::future::poll_fn;
    use async_std::task::block_on;

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