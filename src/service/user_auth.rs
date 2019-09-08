mod request;
mod method;
mod success;
mod failure;

pub use self::request::*;
pub use self::method::*;
pub use self::success::*;
pub use self::failure::*;

use crate::codec::*;
use crate::transport::*;

use async_std::io::{Read, Write};
use futures::io::{AsyncRead, AsyncWrite};
use std::convert::{From, TryInto};
use std::time::{Duration, Instant};


pub async fn authenticate<T: TransportStream>(transport: &mut Transport<T>) -> Result<(), UserAuthError> {
    println!("CONNECTED: {:?}", "ASD");
    let req = ServiceRequest::user_auth();
    transport.send(&req).await?;
    transport.flush().await?;
    let res: ServiceAccept<'_> = transport.receive().await?;
    let req: Request = Request {
        user_name: "lpetersen",
        service_name: "ssh-connection",
        method: Method::Password(Password("1234567890".into()))
    };
    println!("ABC {:?}", res);
    transport.send(&req).await?;
    transport.flush().await?;
    match transport.receive().await? {
        E2::A(x) => {
            let _: Success = x;
            println!("{:?}", x);
        },
        E2::B(x) => {
            let _: Failure = x;
            println!("{:?}", x);
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum UserAuthError {
    TransportError(TransportError)
}

impl From<TransportError> for UserAuthError {
    fn from(e: TransportError) -> Self {
        Self::TransportError(e)
    }
}
