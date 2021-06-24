use shellfish::connection::channel::DirectTcpIp;
use shellfish::connection::channel::DirectTcpIpParams;
use shellfish::connection::channel::DirectTcpIpRequest;
use shellfish::connection::channel::OpenFailure;
use shellfish::connection::global::Global;
use shellfish::connection::global::GlobalRequest;
use shellfish::connection::global::GlobalRequestWantReply;
use shellfish::connection::global::GlobalWantReply;
use shellfish::connection::Connection;
use shellfish::connection::ConnectionConfig;
use shellfish::connection::ConnectionError;
use shellfish::connection::ConnectionHandler;
use shellfish::transport::GenericTransport;
use shellfish::transport::TestTransport;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio::time::{timeout, Timeout};

/// Create a coupled pair of connection objects (over in-memory transport)
fn new_connection<F>(f: F) -> (Connection, Connection)
where
    F: FnOnce(&Connection) -> Box<dyn ConnectionHandler>,
{
    let config = Arc::new(ConnectionConfig::default());
    let (t1, t2) = TestTransport::new();
    let c1 = Connection::new(&config, GenericTransport::from(t1), f);
    let c2 = Connection::new(&config, GenericTransport::from(t2), |_| Box::new(()));
    (c1, c2)
}

/// Timeout a minimal amount of time
fn timeout_1ms<T, F: Future<Output = T>>(f: F) -> Timeout<F> {
    timeout(Duration::from_millis(1), f)
}

#[tokio::test]
async fn test_connection_drop() {
    let (mut c1, c2) = new_connection(|_| Box::new(()));

    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
    drop(c2);
    c1.closed().await;
    assert!(c1.check().is_err());
}

#[tokio::test]
async fn test_connection_close() {
    let (mut c1, mut c2) = new_connection(|_| Box::new(()));

    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
    c2.close();
    c1.closed().await;
    c2.closed().await;
    assert!(c1.check().is_err());
    assert!(c2.check().is_err());
}

#[tokio::test]
async fn test_connection_handler_poll_ready_causes_close() {
    pub struct TestHandler(oneshot::Receiver<()>);
    impl ConnectionHandler for TestHandler {
        fn poll(&mut self, cx: &mut Context) -> Poll<()> {
            Future::poll(Pin::new(&mut self.0), cx).map(drop)
        }
    }

    let (s, r) = oneshot::channel::<()>();
    let h = TestHandler(r);
    let (mut c1, mut c2) = new_connection(|_| Box::new(h));

    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
    s.send(()).unwrap();
    c1.closed().await;
    c2.closed().await;
    assert!(c1.check().is_err());
    assert!(c2.check().is_err());
}

#[tokio::test]
async fn test_connection_handler_on_error_gets_called_on_error() {
    pub struct TestHandler(oneshot::Sender<ConnectionError>);
    impl ConnectionHandler for TestHandler {
        fn on_error(self: Box<Self>, e: &ConnectionError) {
            self.0.send(e.clone()).unwrap()
        }
    }

    let (s, mut r) = oneshot::channel::<ConnectionError>();
    let h = TestHandler(s);
    let (mut c1, mut c2) = new_connection(|_| Box::new(h));

    assert!(r.try_recv().is_err()); // not yet sent
    c1.close();
    c2.close();
    c1.closed().await;
    c2.closed().await;
    assert!(r.try_recv().is_ok());
    assert!(c1.check().is_err());
    assert!(c2.check().is_err());
}

#[tokio::test]
async fn test_connection_check_with_keepalive() {
    pub struct TestHandler(Option<oneshot::Sender<String>>);
    impl ConnectionHandler for TestHandler {
        fn on_request_want_reply(&mut self, req: GlobalRequestWantReply) {
            if let Some(s) = self.0.take() {
                let _ = s.send(String::from(req.name()));
            }
        }
    }

    let (s, mut r) = oneshot::channel::<String>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    assert!(r.try_recv().is_err()); // not yet sent
    assert!(c2.check_with_keepalive().await.is_ok());
    assert_eq!(r.try_recv().unwrap(), "keepalive@openssh.com");
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_request() {
    pub struct TestRequest;
    impl Global for TestRequest {
        const NAME: &'static str = "test-request@example.com";
        type RequestData = Vec<u8>;
    }

    pub struct TestHandler(Option<oneshot::Sender<(String, Vec<u8>)>>);
    impl ConnectionHandler for TestHandler {
        fn on_request(&mut self, req: GlobalRequest) {
            if let Some(s) = self.0.take() {
                let name = req.name().to_string();
                let data = req.data().clone();
                let _ = s.send((name, data));
            }
        }
    }

    let (s, r) = oneshot::channel::<(String, Vec<u8>)>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let data = vec![1, 2, 3, 4];
    assert!(c2.request::<TestRequest>(&data).await.is_ok());
    let (name, data) = r.await.unwrap();
    assert_eq!(name, "test-request@example.com");
    assert_eq!(data, [1, 2, 3, 4]);
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_request_want_reply_accept() {
    pub struct TestRequest;
    impl Global for TestRequest {
        const NAME: &'static str = "test-request@example.com";
        type RequestData = Vec<u8>;
    }
    impl GlobalWantReply for TestRequest {
        type ResponseData = Vec<u8>;
    }

    pub struct TestHandler(Option<oneshot::Sender<(String, Vec<u8>)>>);
    impl ConnectionHandler for TestHandler {
        fn on_request_want_reply(&mut self, req: GlobalRequestWantReply) {
            if let Some(s) = self.0.take() {
                let name = req.name().to_string();
                let data = req.data().clone();
                let _ = s.send((name, data));
                req.accept(vec![5, 6, 7, 8]);
            }
        }
    }

    let (s, r) = oneshot::channel::<(String, Vec<u8>)>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let data = vec![1, 2, 3, 4];
    let fata = c2.request_want_reply::<TestRequest>(&data).await.unwrap();
    assert_eq!(fata, Ok(vec![5, 6, 7, 8]));
    let (name, data) = r.await.unwrap();
    assert_eq!(name, "test-request@example.com");
    assert_eq!(data, [1, 2, 3, 4]);
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_request_want_reply_reject() {
    pub struct TestRequest;
    impl Global for TestRequest {
        const NAME: &'static str = "test-request@example.com";
        type RequestData = Vec<u8>;
    }
    impl GlobalWantReply for TestRequest {
        type ResponseData = Vec<u8>;
    }

    pub struct TestHandler(Option<oneshot::Sender<(String, Vec<u8>)>>);
    impl ConnectionHandler for TestHandler {
        fn on_request_want_reply(&mut self, req: GlobalRequestWantReply) {
            if let Some(s) = self.0.take() {
                let name = req.name().to_string();
                let data = req.data().clone();
                let _ = s.send((name, data));
                req.reject();
            }
        }
    }

    let (s, r) = oneshot::channel::<(String, Vec<u8>)>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let data = vec![1, 2, 3, 4];
    assert!(c2
        .request_want_reply::<TestRequest>(&data)
        .await
        .unwrap()
        .is_err());
    let (name, data) = r.await.unwrap();
    assert_eq!(name, "test-request@example.com");
    assert_eq!(data, [1, 2, 3, 4]);
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_direct_tcp_ip_open_reject() {
    pub struct TestHandler(Option<oneshot::Sender<DirectTcpIpParams>>);
    impl ConnectionHandler for TestHandler {
        fn on_direct_tcpip_request(&mut self, req: DirectTcpIpRequest) {
            if let Some(s) = self.0.take() {
                let data = req.params().clone();
                let _ = s.send(data);
                req.reject(OpenFailure::OPEN_CONNECT_FAILED);
            }
        }
    }

    let (s, r) = oneshot::channel::<DirectTcpIpParams>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let req = DirectTcpIpParams {
        dst_host: "example.com".into(),
        dst_port: 8080,
        src_addr: std::net::Ipv4Addr::LOCALHOST.into(),
        src_port: 1234,
    };

    let err = c2
        .open_direct_tcpip(req.clone())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(err, OpenFailure::OPEN_CONNECT_FAILED);

    let data = r.await.unwrap();
    assert_eq!(data.dst_host, req.dst_host);
    assert_eq!(data.dst_port, req.dst_port);
    assert_eq!(data.src_addr, req.src_addr);
    assert_eq!(data.src_port, req.src_port);

    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_direct_tcp_ip_open_accept() {
    pub struct TestHandler(Option<oneshot::Sender<DirectTcpIp>>);
    impl ConnectionHandler for TestHandler {
        fn on_direct_tcpip_request(&mut self, req: DirectTcpIpRequest) {
            if let Some(s) = self.0.take() {
                let _ = s.send(req.accept());
            }
        }
    }

    let (s, r) = oneshot::channel::<DirectTcpIp>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let req = DirectTcpIpParams {
        dst_host: "example.com".into(),
        dst_port: 8080,
        src_addr: std::net::Ipv4Addr::LOCALHOST.into(),
        src_port: 1234,
    };

    let d1: DirectTcpIp = c2.open_direct_tcpip(req.clone()).await.unwrap().unwrap();
    let d2 = r.await.unwrap();

    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());

    drop(d1);
    drop(d2);
}

#[tokio::test]
async fn test_connection_direct_tcp_ip_close_by_drop() {
    pub struct TestHandler(Option<oneshot::Sender<DirectTcpIp>>);
    impl ConnectionHandler for TestHandler {
        fn on_direct_tcpip_request(&mut self, req: DirectTcpIpRequest) {
            if let Some(s) = self.0.take() {
                let _ = s.send(req.accept());
            }
        }
    }

    let (s, r) = oneshot::channel::<DirectTcpIp>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let req = DirectTcpIpParams {
        dst_host: "example.com".into(),
        dst_port: 8080,
        src_addr: std::net::Ipv4Addr::LOCALHOST.into(),
        src_port: 1234,
    };

    let mut d1: DirectTcpIp = c2.open_direct_tcpip(req.clone()).await.unwrap().unwrap();
    let d2 = r.await.unwrap();

    // Test that read blocks if channel is open
    let mut buf = [0u8; 3];
    let x = timeout_1ms(d1.read(&mut buf)).await;
    assert!(x.is_err());

    // Now drop channel 2 and repeat (now expecting unexpected eof)
    drop(d2);
    let mut buf = [0u8; 3];
    let x = timeout_1ms(d1.read(&mut buf)).await;
    assert!(x.is_ok());
    let x = x.unwrap();
    assert_eq!(x.unwrap_err().kind(), std::io::ErrorKind::UnexpectedEof);

    // Check that connection is still healthy
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_direct_tcp_ip_shutdown() {
    pub struct TestHandler(Option<oneshot::Sender<DirectTcpIp>>);
    impl ConnectionHandler for TestHandler {
        fn on_direct_tcpip_request(&mut self, req: DirectTcpIpRequest) {
            if let Some(s) = self.0.take() {
                let _ = s.send(req.accept());
            }
        }
    }

    let (s, r) = oneshot::channel::<DirectTcpIp>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let req = DirectTcpIpParams {
        dst_host: "example.com".into(),
        dst_port: 8080,
        src_addr: std::net::Ipv4Addr::LOCALHOST.into(),
        src_port: 1234,
    };

    let mut d1: DirectTcpIp = c2.open_direct_tcpip(req.clone()).await.unwrap().unwrap();
    let mut d2 = r.await.unwrap();

    // Test that read blocks if channel is open
    let mut buf = [0u8; 3];
    let x = timeout_1ms(d1.read(&mut buf)).await;
    assert!(x.is_err());

    // Now shutdown channel 2 and repeat (expecting to read 0 bytes)
    assert!(d2.shutdown().await.is_ok());
    let mut buf = [0u8; 3];
    let x = timeout_1ms(d1.read(&mut buf)).await;
    assert!(x.is_ok());
    let x = x.unwrap();
    assert_eq!(x.unwrap(), 0);

    // Check that connection is still healthy
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}

#[tokio::test]
async fn test_connection_direct_tcp_ip_read_write() {
    pub struct TestHandler(Option<oneshot::Sender<DirectTcpIp>>);
    impl ConnectionHandler for TestHandler {
        fn on_direct_tcpip_request(&mut self, req: DirectTcpIpRequest) {
            if let Some(s) = self.0.take() {
                let _ = s.send(req.accept());
            }
        }
    }

    let (s, r) = oneshot::channel::<DirectTcpIp>();
    let h = TestHandler(Some(s));
    let (c1, c2) = new_connection(|_| Box::new(h));

    let req = DirectTcpIpParams {
        dst_host: "example.com".into(),
        dst_port: 8080,
        src_addr: std::net::Ipv4Addr::LOCALHOST.into(),
        src_port: 1234,
    };

    let mut d1: DirectTcpIp = c2.open_direct_tcpip(req.clone()).await.unwrap().unwrap();
    let mut d2 = r.await.unwrap();

    let buf1 = b"ABCDEF";
    let mut buf2 = [0u8; 6];
    assert_eq!(d1.write(buf1).await.unwrap(), buf1.len());
    assert_eq!(d1.flush().await.is_ok(), true);
    assert_eq!(d2.read(buf2.as_mut()).await.unwrap(), buf1.len());
    assert_eq!(buf1.as_ref(), buf2.as_ref());

    // Check that connection is still healthy
    assert!(c1.check().is_ok());
    assert!(c2.check().is_ok());
}
