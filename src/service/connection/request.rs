use super::*;
use futures::channel::oneshot;
use futures::future::Future;
use futures::ready;
use futures::task::{Context, Poll};
use std::pin::Pin;

pub trait IsRequest: Sized {
    type Response;
    fn try_from(r: Request) -> Option<Transaction<Self>>;
    fn into_request(self, sender: oneshot::Sender<Result<(Self::Response, RequestSender), ConnectionError>>) -> Request;
}

pub enum Request {
    ChannelOpen(Transaction<ChannelOpenRequest>),
    Disconnect(Transaction<DisconnectRequest>),
}

pub struct Transaction<R: IsRequest> {
    pub input: R,
    output: oneshot::Sender<Result<(R::Response, RequestSender), ConnectionError>>,
}

pub struct DisconnectRequest {}

impl Into<Request> for DisconnectRequest {
    fn into(self) -> Request {
        panic!("")
    }
}

impl IsRequest for DisconnectRequest {
    type Response = ();
    fn try_from(r: Request) -> Option<Transaction<Self>> {
        match r {
            Request::Disconnect(x) => Some(x),
            _ => None,
        }
    }
    fn into_request(self, sender: oneshot::Sender<Result<(Self::Response, RequestSender), ConnectionError>>) -> Request {
        Request::Disconnect(Transaction { input: self, output: sender})
    }
}

pub struct ChannelOpenRequest {
    pub initial_window_size: u32,
    pub max_packet_size: u32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ChannelOpenFailure {
    pub reason: Reason,
}

impl IsRequest for ChannelOpenRequest {
    type Response = Result<Session, ChannelOpenFailure>;
    fn try_from(r: Request) -> Option<Transaction<Self>> {
        match r {
            Request::ChannelOpen(x) => Some(x),
            _ => None,
        }
    }
    fn into_request(self, sender: oneshot::Sender<Result<(Self::Response, RequestSender), ConnectionError>>) -> Request {
        Request::ChannelOpen(Transaction { input: self, output: sender})
    }
}

pub fn channel() -> (RequestSender, RequestReceiver) {
    let (s, r) = oneshot::channel();
    (RequestSender::Ready(s), RequestReceiver::Waiting(r))
}

pub enum RequestSender {
    Terminated(ConnectionError),
    Ready(oneshot::Sender<Request>),
    Pending,
}

pub enum RequestReceiver {
    Terminated,
    Waiting(oneshot::Receiver<Request>),
    Processing((bool, Request)),
}

impl RequestSender {
    pub async fn request<R: IsRequest>(&mut self, req: R) -> Result<R::Response, ConnectionError> {
        let (s, r) = oneshot::channel();
        match std::mem::replace(self, Self::Pending) {
            Self::Terminated(e) => {
                *self = Self::Terminated(e);
                Err(e)
            },
            Self::Ready(x) => {
                x.send(IsRequest::into_request(req, s)).map_err(|_| ConnectionError::RequestReceiverDropped)?;
                let (response, sender) = r.await.map_err(|_| ConnectionError::RequestReceiverDropped)??;
                *self = sender;
                Ok(response)
            }
            Self::Pending => panic!("illegal state"),
        }
    }
}

impl RequestReceiver {
    pub fn poll(&mut self, cx: &mut Context) -> Poll<Result<&Request, ConnectionError>> {
        loop {
            match self {
                Self::Terminated => return Poll::Ready(Err(ConnectionError::Terminated)),
                Self::Waiting(r) => {
                    *self = Self::Processing((
                        false,
                        ready!(Pin::new(r).poll(cx))
                            .map_err(|_| ConnectionError::RequestSenderDropped)?,
                    ))
                }
                Self::Processing((accepted, r)) if !*accepted => return Poll::Ready(Ok(r)),
                _ => return Poll::Pending,
            }
        }
    }

    pub fn accept(&mut self) {
        match self {
            Self::Processing(x) if !x.0 => {
                x.0 = true;
            }
            _ => (),
        }
    }

    pub fn complete<F, R, T>(&mut self, f: F) -> Result<T, ConnectionError>
    where
        R: IsRequest,
        F: FnOnce(R) -> Result<(R::Response, T), ConnectionError>,
    {
        let (s, r) = oneshot::channel();
        let sender = RequestSender::Ready(s);
        let receiver = RequestReceiver::Waiting(r);
        match std::mem::replace(self, receiver) {
            Self::Terminated => return Err(ConnectionError::Terminated),
            Self::Processing((true, x)) => match IsRequest::try_from(x) {
                None => return Err(ConnectionError::RequestUnexpectedResponse),
                Some(r) => {
                    let (response, t) = f(r.input)?;
                    r.output
                        .send(Ok((response, sender)))
                        .map_err(|_| ConnectionError::RequestSenderDropped)?;
                    Ok(t)
                }
            },
            _ => return Err(ConnectionError::RequestUnexpectedResponse),
        }
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        match std::mem::replace(self, Self::Terminated) {
            Self::Processing((_, x)) => match x {
                Request::ChannelOpen(x) => x.output.send(Err(e)).unwrap_or(()),
                Request::Disconnect(x) => x.output.send(Err(e)).unwrap_or(()),
            },
            _ => (),
        }
    }
}
