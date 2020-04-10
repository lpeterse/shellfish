use async_std::future::Future;
use async_std::task::*;
use core::pin::Pin;
use std::cell::Cell;
use std::sync::{Arc, Mutex};

pub fn new<T>() -> (Sender<T>, Receiver<T>) {
    let st = State {
        tx: None,
        rx: None,
        dropped: false,
        value: None,
    };
    let x = Arc::new(Mutex::new(st));
    (Sender(x.clone()), Receiver(x))
}

type Channel<T> = Arc<Mutex<State<T>>>;

#[derive(Debug)]
struct State<T> {
    tx: Option<Waker>,
    rx: Option<Waker>,
    dropped: bool,
    value: Option<T>,
}

#[derive(Debug)]
pub struct Sender<T>(Channel<T>);

impl<T> Sender<T> {
    pub async fn send(&self, t: T) -> Option<()> {
        SendFuture {
            tx: self,
            t: Cell::new(Some(t)),
        }
        .await
    }

    pub fn poll_send(&self, cx: &mut Context, t: T) -> Poll<Option<()>> {
        if let Ok(mut st) = self.0.lock() {
            if st.dropped {
                return Poll::Ready(None);
            }
            if st.value.is_none() {
                st.value = Some(t);
                st.tx = None;
                st.rx.take().map(Waker::wake).unwrap_or(());
                return Poll::Ready(Some(()));
            }
            if let Some(ref waker) = st.tx {
                if waker.will_wake(cx.waker()) {
                    return Poll::Pending;
                }
            }
            st.tx = Some(cx.waker().clone());
            return Poll::Pending;
        } else {
            return Poll::Ready(None);
        }
    }
}

#[derive(Debug)]
pub struct Receiver<T>(Channel<T>);

impl<T> Receiver<T> {
    pub fn poll_receive(&self, cx: &mut Context) -> Poll<Option<T>> {
        if let Ok(mut st) = self.0.lock() {
            if let Some(v) = st.value.take() {
                st.rx = None;
                st.tx.take().map(Waker::wake).unwrap_or(());
                return Poll::Ready(Some(v));
            }
            if st.dropped {
                return Poll::Ready(None);
            }
            if let Some(ref waker) = st.rx {
                if waker.will_wake(cx.waker()) {
                    return Poll::Pending;
                }
            }
            st.rx = Some(cx.waker().clone());
            return Poll::Pending;
        } else {
            return Poll::Ready(None);
        }
    }
}

struct SendFuture<'a, T: Sized> {
    tx: &'a Sender<T>,
    t: Cell<Option<T>>,
}

impl<'a, T> Future for SendFuture<'a, T> {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<()>> {
        if let Ok(mut st) = self.tx.0.lock() {
            if st.dropped {
                return Poll::Ready(None);
            }
            if st.value.is_none() {
                st.value = self.t.replace(None);
                st.tx = None;
                st.rx.take().map(Waker::wake).unwrap_or(());
                return Poll::Ready(Some(()));
            }
            if let Some(ref waker) = st.tx {
                if waker.will_wake(cx.waker()) {
                    return Poll::Pending;
                }
            }
            st.tx = Some(cx.waker().clone());
            return Poll::Pending;
        } else {
            return Poll::Ready(None);
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Ok(mut st) = self.0.lock() {
            st.dropped = true;
            st.tx.take().map(Waker::wake).unwrap_or(());
            st.rx.take().map(Waker::wake).unwrap_or(());
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if let Ok(mut st) = self.0.lock() {
            st.dropped = true;
            st.tx.take().map(Waker::wake).unwrap_or(());
            st.rx.take().map(Waker::wake).unwrap_or(());
        }
    }
}
