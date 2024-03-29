use std::io::Result;
use std::net::{IpAddr, SocketAddr};

use crate::util::socket::{flush, read_exact, write_all, Socket};

pub async fn serve<S: Socket>(mut sock: S) -> Result<ConnectRequest<S>> {
    let e: std::io::ErrorKind = std::io::ErrorKind::InvalidInput;
    let mut buf: [u8; 255] = [0; 255];
    // Read the first 2 bytes: Version and number of auth methods
    read_exact(&mut sock, &mut buf[..2]).await?;
    buf.get(0).filter(|x| **x == Version::V5.0).ok_or(e)?;
    let n: usize = *buf.get(1).ok_or(e)? as usize;
    // Read the announced number of auth methods
    read_exact(&mut sock, &mut buf[..n]).await?;
    // If the client requests no authentication, send positive reply else terminate
    let pred = |x: &&u8| **x == AuthMethod::NO_AUTHENTICATION_REQUIRED.0;
    if buf[..n].iter().find(pred).is_some() {
        let response = [Version::V5.0, AuthMethod::NO_AUTHENTICATION_REQUIRED.0];
        write_all(&mut sock, response.as_ref()).await?;
        flush(&mut sock).await?;
    } else {
        let response = [Version::V5.0, AuthMethod::NO_ACCEPTABLE_METHODS.0];
        write_all(&mut sock, response.as_ref()).await?;
        flush(&mut sock).await?;
        Err(e)?
    }
    // Read the first 4 bytes of a request
    read_exact(&mut sock, &mut buf[..4]).await?;
    buf.get(0).filter(|x| **x == Version::V5.0).ok_or(e)?;
    buf.get(2).filter(|x| **x == 0).ok_or(e)?;
    // Terminate unless this is connect request
    if Command(*buf.get(1).ok_or(e)?) != Command::CONNECT {
        let response = [Version::V5.0, Reply::COMMAND_NOT_SUPPORTED.0, 0];
        write_all(&mut sock, response.as_ref()).await?;
        flush(&mut sock).await?;
        Err(e)?;
    }
    // Read the remaining request based on the address type
    Ok(match AddrType(*buf.get(3).ok_or(e)?) {
        AddrType::IP_V4 => {
            let mut addr: [u8; 4] = [0; 4];
            let mut port: [u8; 2] = [0; 2];
            read_exact(&mut sock, &mut addr).await?;
            read_exact(&mut sock, &mut port).await?;
            let host = Host::Addr(IpAddr::V4(addr.into()));
            let port = u16::from_be_bytes(port);
            ConnectRequest::new(sock, host, port)
        }
        AddrType::IP_V6 => {
            let mut addr: [u8; 16] = [0; 16];
            let mut port: [u8; 2] = [0; 2];
            read_exact(&mut sock, &mut addr).await?;
            read_exact(&mut sock, &mut port).await?;
            let host = Host::Addr(IpAddr::V6(addr.into()));
            let port = u16::from_be_bytes(port);
            ConnectRequest::new(sock, host, port)
        }
        AddrType::DOMAINNAME => {
            let mut n = [0];
            let mut port: [u8; 2] = [0; 2];
            read_exact(&mut sock, &mut n).await?;
            let n = *n.get(0).ok_or(e)? as usize;
            let mut name = Vec::with_capacity(n);
            name.resize_with(n, Default::default);
            read_exact(&mut sock, &mut name).await?;
            read_exact(&mut sock, &mut port).await?;
            let host = Host::Name(String::from_utf8(name).ok().ok_or(e)?);
            let port = u16::from_be_bytes(port);
            ConnectRequest::new(sock, host, port)
        }
        _ => Err(e)?,
    })
}

#[derive(Debug)]
pub struct ConnectRequest<S: Socket> {
    sock: S,
    host: Host,
    port: u16,
}

impl<S: Socket> ConnectRequest<S> {
    fn new(sock: S, host: Host, port: u16) -> Self {
        Self { sock, host, port }
    }

    pub fn host(&self) -> &Host {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn accept(self, bind_addr: SocketAddr) -> Result<S> {
        let mut sock = self.sock;
        let t = match bind_addr.ip() {
            IpAddr::V4(_) => AddrType::IP_V4,
            IpAddr::V6(_) => AddrType::IP_V6,
        };
        let x = [Version::V5.0, Reply::SUCCEEDED.0, 0, t.0];
        write_all(&mut sock, &x).await?;
        match bind_addr.ip() {
            IpAddr::V4(addr) => write_all(&mut sock, &addr.octets()).await?,
            IpAddr::V6(addr) => write_all(&mut sock, &addr.octets()).await?,
        }
        write_all(&mut sock, &bind_addr.port().to_be_bytes()).await?;
        flush(&mut sock).await?;
        Ok(sock)
    }
}

#[derive(Debug)]
pub enum Host {
    Addr(IpAddr),
    Name(String),
}

impl std::string::ToString for Host {
    fn to_string(&self) -> String {
        match self {
            Self::Addr(x) => x.to_string(),
            Self::Name(x) => x.clone(),
        }
    }
}

struct Version(u8);

impl Version {
    pub const V5: Self = Self(5);
}

struct Reply(u8);

#[allow(dead_code)]
impl Reply {
    pub const SUCCEEDED: Self = Self(0);
    pub const GENERAL_SOCKS_SERVER_FAILURE: Self = Self(1);
    pub const CONNECTION_NOT_ALLOWED_BY_RULESET: Self = Self(2);
    pub const NETWORK_UNREACHABLE: Self = Self(3);
    pub const HOST_UNREACHABLE: Self = Self(4);
    pub const CONNECTION_REFUSED: Self = Self(5);
    pub const TTL_EXPIRED: Self = Self(6);
    pub const COMMAND_NOT_SUPPORTED: Self = Self(7);
    pub const ADDRESS_TYPE_NOT_SUPPORTED: Self = Self(8);
}

#[derive(PartialEq, Eq)]
struct AddrType(u8);

impl AddrType {
    const IP_V4: Self = Self(1);
    const IP_V6: Self = Self(4);
    const DOMAINNAME: Self = Self(3);
}

struct AuthMethod(u8);

impl AuthMethod {
    const NO_AUTHENTICATION_REQUIRED: Self = Self(0);
    const NO_ACCEPTABLE_METHODS: Self = Self(255);
}

#[derive(PartialEq, Eq)]
struct Command(u8);

impl Command {
    const CONNECT: Self = Self(1);
}
