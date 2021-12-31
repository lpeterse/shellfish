use tokio::net::{TcpListener, TcpStream};

pub struct Socket;

impl Socket {
    pub async fn new_tcp() -> Result<(TcpStream, TcpStream), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("localhost:0").await?;
        let addr = listener.local_addr()?;
        let s1 = TcpStream::connect(addr).await?;
        let (s2, _) = listener.accept().await?;
        Ok((s1, s2))
    }
}
