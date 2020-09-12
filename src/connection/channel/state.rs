use super::stream::Stream;
use super::*;

use crate::util::assume;
use crate::util::buffer::Buffer;

use async_std::task::Context;
use async_std::task::Waker;

#[derive(Debug)]
pub struct ChannelState {
    pub max_buffer_size: u32,

    pub lid: u32,
    pub lws: u32,
    pub lmps: u32,
    pub leof: bool,
    pub leof_sent: bool,
    pub lclose: bool,
    pub lclose_sent: bool,

    pub rid: u32,
    pub rws: u32,
    pub rmps: u32,
    pub reof: bool,
    pub rclose: bool,

    pub std: Stream,
    pub ext: Option<Box<Stream>>,

    pub inner_task_wake: bool,
    pub inner_task_waker: Option<Waker>,
    pub outer_task_wake: bool,
    pub outer_task_waker: Option<Waker>,

    pub error: Option<ConnectionError>,
}

impl ChannelState {
    pub fn new(lid: u32, lmbs: u32, lmps: u32, rid: u32, rws: u32, rmps: u32, ext: bool) -> Self {
        Self {
            max_buffer_size: lmbs,

            lid,
            lws: lmbs,
            lmps,
            leof: false,
            leof_sent: false,
            lclose: false,
            lclose_sent: false,

            rid,
            rws,
            rmps,
            reof: false,
            rclose: false,

            std: Stream::default(),
            ext: assume(ext).map(|_| Box::new(Stream::default())),

            inner_task_wake: false,
            inner_task_waker: None,
            outer_task_wake: false,
            outer_task_waker: None,

            error: None,
        }
    }

    pub fn std_rx(&mut self) -> &mut Buffer {
        &mut self.std.rx
    }

    pub fn std_tx(&mut self) -> &mut Buffer {
        &mut self.std.tx
    }

    pub fn outer_task_waker(&mut self) -> Option<Waker> {
        if self.outer_task_wake {
            self.outer_task_waker.take()
        } else {
            None
        }
    }

    pub fn inner_task_waker(&mut self) -> Option<Waker> {
        if self.inner_task_wake {
            self.inner_task_waker.take()
        } else {
            None
        }
    }

    pub fn register_outer_task(&mut self, cx: &mut Context) {
        self.outer_task_wake = false;
        self.outer_task_waker = Some(cx.waker().clone())
    }

    pub fn poll_inner_task_woken(&mut self, cx: &mut Context) -> Poll<()> {
        self.inner_task_waker = Some(cx.waker().clone());
        if self.inner_task_wake {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn local_window_adjust(&mut self) -> Option<u32> {
        let threshold = self.max_buffer_size / 2;
        if self.lws < threshold {
            let mut buffered = self.std.rx.len() as u32;
            if let Some(ref ext) = self.ext {
                buffered += ext.rx.len() as u32;
            }
            if buffered < threshold {
                return Some(self.max_buffer_size - std::cmp::max(self.lws, buffered));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_std::future::poll_fn;
    use async_std::task::block_on;

    #[test]
    fn test_channel_state_inner_new_01() {
        let st = ChannelState::new(1, 2, 3, 4, 5, 6, false);
        assert_eq!(st.lid, 1);
        assert_eq!(st.lws, 2);
        assert_eq!(st.max_buffer_size, 2);
        assert_eq!(st.lmps, 3);
        assert_eq!(st.leof, false);
        assert_eq!(st.leof_sent, false);
        assert_eq!(st.lclose, false);
        assert_eq!(st.lclose_sent, false);
        assert_eq!(st.rid, 4);
        assert_eq!(st.rws, 5);
        assert_eq!(st.rmps, 6);
        assert_eq!(st.reof, false);
        assert_eq!(st.rclose, false);
        assert_eq!(st.ext.is_some(), false);
        assert_eq!(st.inner_task_wake, false);
        assert_eq!(st.outer_task_wake, false);
        assert_eq!(st.error.is_some(), false);
    }

    #[test]
    fn test_channel_state_inner_new_02() {
        let st = ChannelState::new(1, 2, 3, 4, 5, 6, true);
        assert_eq!(st.ext.is_some(), true);
    }

    #[test]
    fn test_channel_state_inner_register_outer_task_01() {
        let mut st = ChannelState::new(1, 2, 3, 4, 5, 6, false);
        st.outer_task_wake = true;
        block_on(poll_fn(|cx| {
            st.register_outer_task(cx);
            assert_eq!(st.outer_task_wake, false);
            assert_eq!(st.outer_task_waker.is_some(), true);
            Poll::Ready(())
        }));
    }

    #[test]
    fn test_channel_state_inner_inner_task_waker_01() {
        let mut st = ChannelState::new(1, 2, 3, 4, 5, 6, false);
        block_on(poll_fn(|cx| {
            assert_eq!(st.inner_task_waker().is_some(), false);
            assert_eq!(st.poll_inner_task_woken(cx), Poll::Pending);
            assert_eq!(st.inner_task_waker().is_some(), false);
            st.inner_task_wake = true;
            assert_eq!(st.inner_task_waker().is_some(), true);
            assert_eq!(st.inner_task_waker().is_some(), false);
            Poll::Ready(())
        }));
    }

    #[test]
    fn test_channel_state_inner_outer_task_waker_01() {
        let mut st = ChannelState::new(1, 2, 3, 4, 5, 6, false);
        block_on(poll_fn(|cx| {
            assert_eq!(st.outer_task_waker().is_some(), false);
            st.register_outer_task(cx);
            assert_eq!(st.outer_task_waker().is_some(), false);
            st.outer_task_wake = true;
            assert_eq!(st.outer_task_waker().is_some(), true);
            assert_eq!(st.outer_task_waker().is_some(), false);
            Poll::Ready(())
        }));
    }
}
