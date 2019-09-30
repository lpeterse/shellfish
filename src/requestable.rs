use futures::channel::{mpsc, oneshot};
use futures::future::{TryFutureExt};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use futures::task::{Context, Poll};
use std::convert::{TryFrom, TryInto};

pub trait Requestable {
    type Request;
    type Response;
    type Error: Copy + From<Error>;
}

pub fn channel<T: Requestable>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (sink, stream) = mpsc::channel(capacity);
    (
        Sender { queue: sink },
        Receiver {
            current: None,
            queue: stream,
        },
    )
}

pub struct Sender<T: Requestable> {
    queue: mpsc::Sender<Transaction<T>>,
}

impl<T: Requestable> Sender<T> {
    pub async fn request<R: TryFrom<T::Response>>(
        &mut self,
        request: T::Request,
    ) -> Result<R, T::Error> {
        let (cmd, response) = Transaction::new(request);
        self.queue.send(cmd).map_err(|_| Error::Canceled).await?;
        let response: T::Response = response.map_err(|_| Error::Canceled).await??;
        match response.try_into() {
            Ok(r) => Ok(r),
            Err(_) => Err(Error::UnexpectedResponse.into()),
        }
    }
}

pub struct Receiver<T: Requestable> {
    current: Option<Transaction<T>>,
    queue: mpsc::Receiver<Transaction<T>>,
}

impl<T: Requestable> Receiver<T> {
    pub fn poll(&mut self, cx: &mut Context) -> Poll<Result<&T::Request, T::Error>> {
        loop {
            match self.current {
                Some(ref cmd) => {
                    if cmd.accepted {
                        return Poll::Pending;
                    } else {
                        return Poll::Ready(cmd.ref_request())
                    }
                }
                None => match self.queue.poll_next_unpin(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(None) => return Poll::Ready(Err(Error::Canceled.into())), // FIXME: Called after None
                    Poll::Ready(Some(cmd)) => self.current = Some(cmd),
                },
            }
        }
    }

    pub fn accept(&mut self) -> Result<(), T::Error> {
        match &mut self.current {
            Some(cmd) => {
                if cmd.accepted {
                    Err(Error::AlreadyAccepted.into())
                } else {
                    cmd.accepted = true;
                    Ok(())
                }
            }
            None => Err(Error::NothingToAccept.into()),
        }
    }

    pub fn take<R: TryFrom<T::Request>>(&mut self) -> Result<R, T::Error> {
        match &mut self.current {
            Some(cmd) => {
                if !cmd.accepted {
                    Err(Error::NotYetAccepted.into())
                } else {
                    cmd.take_request()
                }
            }
            None => Err(Error::NothingToTake.into()),
        }
    }

    pub fn respond<R: Into<T::Response>>(&mut self, response: R) -> Result<(), T::Error> {
        match std::mem::replace(&mut self.current, None) {
            Some(cmd) => {
                if !cmd.accepted {
                    Err(Error::NotYetAccepted.into())
                } else {
                    cmd.respond(response.into());
                    Ok(())
                }
            }
            None => Err(Error::NothingToRespond.into()),
        }
    }

    pub fn terminate(&mut self, e: T::Error) {
        // Stop any further input into the queue.
        self.queue.close();
        // Notify the current command (if present).
        std::mem::replace(&mut self.current, None)
            .map(|cmd| cmd.response.send(Err(e)).unwrap_or(()));
        // Nofity all pending command issuers about the error condition.
        loop {
            match self.queue.try_next() {
                Ok(Some(cmd)) => cmd.reject(e),
                _ => break,
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Error {
    Canceled,
    NothingToAccept,
    AlreadyAccepted,
    NotYetAccepted,
    NothingToTake,
    NothingToRespond,
    UnexpectedRequest,
    UnexpectedResponse,
}

struct Transaction<T: Requestable> {
    accepted: bool,
    request: Option<T::Request>,
    response: oneshot::Sender<Result<T::Response, T::Error>>,
}

impl<T: Requestable> Transaction<T> {
    fn new(request: T::Request) -> (Self, oneshot::Receiver<Result<T::Response, T::Error>>) {
        let (response, r) = oneshot::channel();
        let cmd = Transaction {
            accepted: false,
            request: Some(request),
            response,
        };
        (cmd, r)
    }

    fn ref_request(&self) -> Result<&T::Request, T::Error> {
        match &self.request {
            None => Err(Error::NothingToTake.into()),
            Some(r) => Ok(r),
        }
    }

    fn take_request<R: TryFrom<T::Request>>(&mut self) -> Result<R, T::Error> {
        match std::mem::replace(&mut self.request, None) {
            None => Err(Error::NothingToTake.into()),
            Some(r) => match r.try_into() {
                Ok(r) => Ok(r),
                Err(_) => Err(Error::UnexpectedRequest.into())
            }
        }
    }

    fn respond(self, response: T::Response) {
        self.response.send(Ok(response)).unwrap_or(())
    }

    fn reject(self, error: T::Error) {
        self.response.send(Err(error)).unwrap_or(())
    }
}
