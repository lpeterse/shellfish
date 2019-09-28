
use futures::task::{AtomicWaker, Context, Poll};
use std::sync::{Mutex,Arc};

pub struct Exchange<Req,Res> {
    state: State<Req,Res>,
    client: AtomicWaker,
    server: AtomicWaker,
}

pub enum State<Req,Res> {
    Ready,
    Request(Req),
    Processing,
    Response(Res),
    Dropped,
}

impl <Req,Res> Exchange<Req,Res> {
    pub fn new() -> (Client<Req,Res>, Server<Req,Res>) {
        let x = Arc::new(Mutex::new(Exchange {
            state: State::Ready,
            client: AtomicWaker::new(),
            server: AtomicWaker::new(),
        }));
        (Client(x.clone()), Server(x))
    }
}

pub enum ExchangeError {
    Dropped,
    ProtocolViolation,
}

pub struct Client<Req,Res> (Arc<Mutex<Exchange<Req,Res>>>);

impl <Req,Res> Drop for Client<Req,Res> {
    fn drop(&mut self) {
        let mut x = self.0.lock().unwrap();
        x.state = State::Dropped;
        x.server.wake();
    }
}

impl <Req,Res> Client<Req,Res> {
    pub fn request(&mut self, request: Req) -> Result<(),ExchangeError> {
        let mut x = self.0.lock().unwrap();
        match x.state {
            State::Dropped => Err(ExchangeError::Dropped),
            State::Ready => {
                x.state = State::Request(request);
                x.server.wake();
                Ok(())
            },
            _ => Err(ExchangeError::ProtocolViolation)
        }
    }

    pub fn poll_response(&mut self, cx: &mut Context) -> Poll<Result<Res,ExchangeError>> {
        let mut x = self.0.lock().unwrap();
        x.server.register(cx.waker());
        match x.state {
            State::Ready => return Poll::Pending,
            State::Request(_) => return Poll::Pending,
            State::Processing => return Poll::Pending,
            State::Response(_) => match std::mem::replace(&mut x.state, State::Ready) {
                State::Response(r) => return Poll::Ready(Ok(r)),
                _ => panic!("impossible")
            }
            State::Dropped => return Poll::Ready(Err(ExchangeError::Dropped))
        }
    }
}

pub struct Server<Req,Res> (Arc<Mutex<Exchange<Req,Res>>>);

impl <Req,Res> Drop for Server<Req,Res> {
    fn drop(&mut self) {
        let mut x = self.0.lock().unwrap();
        x.state = State::Dropped;
        x.client.wake();
    }
}

impl <Req,Res> Server<Req,Res> {
    pub fn poll_request(&mut self, cx: Context) -> Poll<Result<Req,ExchangeError>> {
        let mut x = self.0.lock().unwrap();
        x.server.register(cx.waker());
        match x.state {
            State::Ready => return Poll::Pending,
            State::Request(_) => match std::mem::replace(&mut x.state, State::Ready) {
                State::Request(r) => return Poll::Ready(Ok(r)),
                _ => panic!("impossible")
            }
            State::Processing => return Poll::Pending,
            State::Response(_) => return Poll::Pending,
            State::Dropped => return Poll::Ready(Err(ExchangeError::Dropped))
        }
    }

    pub fn restore(&mut self, request: Req) -> Result<(), ExchangeError> {
        let mut x = self.0.lock().unwrap();
        match x.state {
            State::Dropped => Err(ExchangeError::Dropped),
            State::Processing => {
                x.state = State::Request(request);
                Ok(())
            },
            _ => Err(ExchangeError::ProtocolViolation)
        }
    }

    pub fn respond(&mut self, response: Res) -> Result<(), ExchangeError> {
        let mut x = self.0.lock().unwrap();
        match x.state {
            State::Dropped => Err(ExchangeError::Dropped),
            State::Processing => {
                x.state = State::Response(response);
                x.client.wake();
                Ok(())
            },
            _ => Err(ExchangeError::ProtocolViolation)
        }
    }
}
