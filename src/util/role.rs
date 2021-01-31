/// This is the distinction between client and server.
pub trait Role: Sized + Unpin + Send + Sync + 'static {}
