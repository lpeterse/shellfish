use super::*;
use crate::util::oneshot;

use async_std::future::Future;
use async_std::task::{ready, Context, Poll};
use std::pin::Pin;

pub(crate) enum Request {
    OpenSession(Transaction<OpenRequest<Session<Client>>>),
    OpenDirectTcpIp(Transaction<OpenRequest<DirectTcpIp>>),
}

pub(crate) struct Transaction<R: IsRequest> {
    pub input: R,
    pub output: oneshot::Sender<Result<(R::Result, RequestSender), ConnectionError>>,
}

pub(crate) struct OpenRequest<T: Channel> {
    pub specific: <T as Channel>::Open,
}

pub(crate) trait IsRequest: Sized {
    type Result: Sized;
    fn try_from(r: Request) -> Option<Transaction<Self>>;
    fn into_request(
        self,
        sender: oneshot::Sender<Result<(Self::Result, RequestSender), ConnectionError>>,
    ) -> Request;
}

impl IsRequest for OpenRequest<Session<Client>> {
    type Result = Result<Session<Client>, ChannelOpenFailureReason>;
    fn try_from(r: Request) -> Option<Transaction<Self>> {
        match r {
            Request::OpenSession(x) => Some(x),
            _ => None,
        }
    }
    fn into_request(
        self,
        sender: oneshot::Sender<Result<(Self::Result, RequestSender), ConnectionError>>,
    ) -> Request {
        Request::OpenSession(Transaction {
            input: self,
            output: sender,
        })
    }
}

impl IsRequest for OpenRequest<DirectTcpIp> {
    type Result = Result<DirectTcpIp, ChannelOpenFailureReason>;
    fn try_from(r: Request) -> Option<Transaction<Self>> {
        match r {
            Request::OpenDirectTcpIp(x) => Some(x),
            _ => None,
        }
    }
    fn into_request(
        self,
        sender: oneshot::Sender<Result<(Self::Result, RequestSender), ConnectionError>>,
    ) -> Request {
        Request::OpenDirectTcpIp(Transaction {
            input: self,
            output: sender,
        })
    }
}

pub(crate) fn channel() -> (RequestSender, RequestReceiver) {
    let (s, r) = oneshot::channel();
    (RequestSender::Ready(s), RequestReceiver::Waiting(r))
}

pub(crate) enum RequestSender {
    Terminated(ConnectionError),
    Ready(oneshot::Sender<Request>),
    Pending,
}

pub(crate) enum RequestReceiver {
    Terminated,
    Waiting(oneshot::Receiver<Request>),
    Processing((bool, Request)),
}

impl RequestSender {
    pub async fn request<R: IsRequest>(&mut self, req: R) -> Result<R::Result, ConnectionError> {
        let (s, r) = oneshot::channel();
        match std::mem::replace(self, Self::Pending) {
            Self::Terminated(e) => {
                *self = Self::Terminated(e);
                Err(e)
            }
            Self::Ready(x) => {
                x.send(IsRequest::into_request(req, s));
                let (result, sender) = r.await.ok_or(ConnectionError::RequestReceiverDropped)??;
                *self = sender;
                Ok(result)
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
                            .ok_or(ConnectionError::RequestSenderDropped)?,
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

    pub fn resolve<R: IsRequest>(&mut self, result: R::Result) -> Result<(), ConnectionError> {
        let (s, r) = oneshot::channel();
        let sender = RequestSender::Ready(s);
        let receiver = RequestReceiver::Waiting(r);
        match std::mem::replace(self, receiver) {
            Self::Terminated => return Err(ConnectionError::Terminated),
            Self::Processing((true, x)) => match <R as IsRequest>::try_from(x) {
                None => return Err(ConnectionError::RequestUnexpectedResponse),
                Some(r) => {
                    r.output.send(Ok((result, sender)));
                    Ok(())
                }
            },
            _ => return Err(ConnectionError::RequestUnexpectedResponse),
        }
    }

    pub fn terminate(&mut self, e: ConnectionError) {
        // FIXME
        /*
        match std::mem::replace(self, Self::Terminated) {
            Self::Processing((_, x)) => match x {
                Request::ChannelOpen(x) => x.output.send(Err(e)),
            },
            _ => (),
        }*/
    }
}
