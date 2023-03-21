use super::pty::Pty;
use crate::connection::{Exit, ExitSignal, ExitStatus, Process};
use std::fs::File;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{ready, Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::pin;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

type TFile = tokio::fs::File;

#[derive(Debug)]
pub struct ChildProcess {
    child: Child,
    child_fds: Fds,
}

impl ChildProcess {
    pub fn spawn(mut cmd: Command, pty: Option<Pty>) -> Result<Self, Error> {
        let cmd = cmd.kill_on_drop(true);

        Ok(if let Some(pty) = pty {
            let s0 = Stdio::from(pty.pts().try_clone()?);
            let s1 = Stdio::from(pty.pts().try_clone()?);
            let s2 = Stdio::from(pty.pts().try_clone()?);
            let m0 = TFile::from_std(File::from(pty.ptm().try_clone()?));
            let m1 = TFile::from_std(File::from(pty.ptm().try_clone()?));
            let m2 = TFile::from_std(File::from(pty.ptm().try_clone()?));
            let child = cmd.stdin(s0).stdout(s1).stderr(s2).spawn()?;
            Self {
                child,
                child_fds: Fds::Pty(m0, m1, m2),
            }
        } else {
            let mut child = cmd
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            let stdin = child.stdin.take().unwrap();
            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            Self {
                child,
                child_fds: Fds::Std(stdin, stdout, stderr),
            }
        })
    }
}

impl Process for ChildProcess {
    fn kill(&mut self, signal: &str) -> Result<(), std::io::Error> {
        let einval = std::io::Error::new(std::io::ErrorKind::InvalidInput, "");
        let esrch = std::io::Error::new(std::io::ErrorKind::NotFound, "");

        let pid = self.child.id().ok_or(esrch)?;
        let sig = match signal {
            "HUP" => Ok(libc::SIGHUP),
            "INT" => Ok(libc::SIGINT),
            "KILL" => Ok(libc::SIGKILL),
            "QUIT" => Ok(libc::SIGQUIT),
            "STOP" => Ok(libc::SIGSTOP),
            "TERM" => Ok(libc::SIGTERM),
            "USR1" => Ok(libc::SIGUSR1),
            "USR2" => Ok(libc::SIGUSR2),
            _ => Err(einval)
        }?;

        unsafe {
            if libc::kill(pid as i32, sig) != 0 {
                return Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    fn poll_stdin_write(
        &mut self,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match &mut self.child_fds {
            Fds::Pty(x, _, _) => Pin::new(x).poll_write(cx, buf),
            Fds::Std(x, _, _) => Pin::new(x).poll_write(cx, buf),
        }
    }

    fn poll_stdin_flush(&mut self, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        match &mut self.child_fds {
            Fds::Pty(x, _, _) => Pin::new(x).poll_flush(cx),
            Fds::Std(x, _, _) => Pin::new(x).poll_flush(cx),
        }
    }

    fn poll_stdin_shutdown(&mut self, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        match &mut self.child_fds {
            Fds::Pty(x, _, _) => Pin::new(x).poll_shutdown(cx),
            Fds::Std(x, _, _) => Pin::new(x).poll_shutdown(cx),
        }
    }

    fn poll_stdout_read(
        &mut self,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<std::io::Result<()>> {
        match &mut self.child_fds {
            Fds::Pty(_, x, _) => Pin::new(x).poll_read(cx, buf),
            Fds::Std(_, x, _) => Pin::new(x).poll_read(cx, buf),
        }
    }

    fn poll_stderr_read(
        &mut self,
        cx: &mut Context,
        buf: &mut ReadBuf,
    ) -> Poll<Result<(), std::io::Error>> {
        match &mut self.child_fds {
            Fds::Pty(_, _, x) => Pin::new(x).poll_read(cx, buf),
            Fds::Std(_, _, x) => Pin::new(x).poll_read(cx, buf),
        }
    }

    fn poll_exit_status(&mut self, cx: &mut Context) -> Poll<Exit> {
        const HUP: &'static str = "HUP";
        let future = self.child.wait();
        pin!(future);
        match ready!(Future::poll(future, cx)) {
            Err(_) => Poll::Ready(Exit::Signal(ExitSignal::new(HUP))),
            Ok(status) => match status.code() {
                None => Poll::Ready(Exit::Signal(ExitSignal::new(HUP))),
                Some(code) => Poll::Ready(Exit::Status(ExitStatus(code as u32))),
            },
        }
    }
}

#[derive(Debug)]
enum Fds {
    Pty(TFile, TFile, TFile),
    Std(ChildStdin, ChildStdout, ChildStderr),
}
