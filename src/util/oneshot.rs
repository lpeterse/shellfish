use core::pin::Pin;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::task::Waker;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Sender<T>(Arc<Mutex<Inner<T>>>);

#[derive(Debug)]
pub struct Receiver<T>(Arc<Mutex<Inner<T>>>);

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let x = Arc::new(Mutex::new(Inner {
        waker: None,
        token: None,
    }));
    (Sender(x.clone()), Receiver(x))
}

#[derive(Debug)]
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

impl<T> Receiver<T> {
    pub fn try_receive(self) -> Option<T> {
        match self.0.lock() {
            Err(_) => None,
            Ok(mut guard) => guard.token.take().flatten(),
        }
    }
}

impl<T: Clone> Receiver<T> {
    pub fn peek(&mut self, cx: &mut Context) -> Poll<Option<T>> {
        match self.0.lock() {
            Err(_) => Poll::Ready(None),
            Ok(mut guard) => match guard.token {
                Some(ref t) => Poll::Ready(t.clone()),
                None => {
                    guard.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            },
        }
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
