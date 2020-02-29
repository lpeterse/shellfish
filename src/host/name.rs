use async_std::net::ToSocketAddrs;

pub trait HostName: ToSocketAddrs {
    fn name(&self) -> String;
}

impl HostName for &str {
    fn name(&self) -> String {
        String::from(*self)
    }
}
