use core::pin::Pin;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::task::{Context, Poll};

pub struct Sender<T>(Arc<Mutex<Inner<T>>>);
pub struct Receiver<T>(Arc<Mutex<Inner<T>>>);

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let x = Arc::new(Mutex::new(Inner {
        waker: None,
        token: None,
    }));
    (Sender(x.clone()), Receiver(x))
}

struct Inner<T> {
    waker: Option<Waker>,
    token: Option<Option<T>>,
}

impl<T> Sender<T> {
    pub fn send(self, x: T) {
        match self.0.lock() {
            Err(_) => None,
            Ok(mut guard) => {
                guard.token = Some(Some(x));
                guard.waker.take()
            }
        }
        .map(Waker::wake)
        .unwrap_or(())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        match self.0.lock() {
            Err(_) => None,
            Ok(mut guard) => {
                if guard.token.is_none() {
                    guard.token = Some(None)
                }
                guard.waker.take()
            }
        }
        .map(Waker::wake)
        .unwrap_or(())
    }
}

impl<T> Future for Receiver<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match self.0.lock() {
            Err(_) => Poll::Ready(None),
            Ok(mut guard) => match guard.token.take() {
                Some(t) => Poll::Ready(t),
                None => {
                    guard.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            },
        }
    }
}