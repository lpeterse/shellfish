mod open;

use super::*;

pub(crate) use self::open::*;

#[derive(Debug)]
pub struct DirectTcpIp(pub(crate) ChannelState);
pub enum DirectTcpIpRequest {}

impl ChannelOpen for DirectTcpIp {
    type Open = DirectTcpIpOpen;
    type Confirmation = ();
}

impl Channel for DirectTcpIp {
    type Request = DirectTcpIpRequest;

    const NAME: &'static str = "direct-tcpip";
}

impl ChannelRequest for DirectTcpIpRequest {
    fn name(&self) -> &'static str {
        unreachable!()
    }
}

impl Encode for DirectTcpIpRequest {
    fn size(&self) -> usize {
        unreachable!()
    }

    fn encode<E: Encoder>(&self, _e: &mut E) {
        unreachable!()
    }
}

impl Drop for DirectTcpIp {
    fn drop(&mut self) {
        let mut x = (self.0).0.lock().unwrap();
        x.close_tx = Some(false);
        x.wake_inner_task();
    }
}

impl Read for DirectTcpIp {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let mut x = (self.0).0.lock().unwrap();
        let read = x.data_in.read(buf);
        if read > 0 {
            x.outer_task = None;
            Poll::Ready(Ok(read))
        } else if x.eof_rx {
            x.outer_task = None;
            Poll::Ready(Ok(0))
        } else {
            x.register_outer_task(cx);
            Poll::Pending
        }
    }
}
