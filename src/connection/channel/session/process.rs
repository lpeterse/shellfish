use super::exit::Exit;
use std::fmt::Debug;
use std::io::Error;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

pub trait Process: Debug + Send + Sync + 'static {
    fn kill(&mut self, signal: &str) -> Result<(), Error>; 
    fn poll_stdin_write(&mut self, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, Error>>;
    fn poll_stdin_flush(&mut self, cx: &mut Context) -> Poll<Result<(), Error>>;
    fn poll_stdin_shutdown(&mut self, cx: &mut Context) -> Poll<Result<(), Error>>;
    fn poll_stdout_read(&mut self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<(), Error>>;
    fn poll_stderr_read(&mut self, cx: &mut Context, buf: &mut ReadBuf) -> Poll<Result<(), Error>>;
    fn poll_exit_status(&mut self, cx: &mut Context) -> Poll<Exit>;
}
