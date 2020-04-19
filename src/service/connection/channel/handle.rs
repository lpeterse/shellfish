use super::super::*;
use super::state::ChannelState;

use crate::transport::TransportLayer;

use async_std::io::{Read, Write};
use std::io::Error;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ChannelHandle(Arc<Mutex<ChannelState>>);

impl ChannelHandle {
    pub fn new(lid: u32, lws: u32, lps: u32, rid: u32, rws: u32, rps: u32) -> Self {
        let x = ChannelState::new(lid, lws, lps, rid, rws, rps);
        Self(Arc::new(Mutex::new(x)))
    }

    pub fn push_data(&mut self, data: &[u8]) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.push_data(data)
    }

    pub fn push_extended_data(&mut self, code: u32, data: &[u8]) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.push_extended_data(code, data)
    }

    pub fn push_eof(&mut self) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.push_eof()
    }

    pub fn push_close(&mut self) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.push_close()
    }

    pub fn push_window_adjust(&mut self, n: u32) -> Result<(), ConnectionError> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.push_window_adjust(n)
    }

    pub fn push_request(&mut self, _request: &[u8]) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_success(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }
    pub fn push_failure(&mut self) -> Result<(), ConnectionError> {
        Ok(())
    }

    pub fn poll<T: TransportLayer>(
        &mut self,
        cx: &mut Context,
        t: &mut T,
    ) -> Poll<Result<(), ConnectionError>> {
        let mut x = self.0.lock().map_err(|_| ConnectionError::Unknown)?;
        x.poll(cx, t)
    }
}

impl Terminate for ChannelHandle {
    fn terminate(&mut self, e: ConnectionError) {
        if let Ok(ref mut x) = self.0.lock() {
            x.terminate(e)
        }
    }
}

impl Read for ChannelHandle {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(self.0.lock().unwrap().deref_mut()).poll_read(cx, buf)
    }
}

impl Write for ChannelHandle {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Pin::new(self.0.lock().unwrap().deref_mut()).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(self.0.lock().unwrap().deref_mut()).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Pin::new(self.0.lock().unwrap().deref_mut()).poll_close(cx)
    }
}
