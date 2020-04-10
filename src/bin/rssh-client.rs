use rssh::client::*;
use rssh::service::connection::future::*;
use rssh::service::connection::*;
use rssh::transport::*;

use async_std::net::TcpStream;
use async_std::stream::StreamExt;

use rssh::util::oneshot;
use std::collections::VecDeque;
use futures_timer::Delay;

async fn foobar(mut conn: Connection) -> Result<(), ConnectionError> {
    /*while let Some(request) = conn.next().await {
        log::warn!("Incoming request: {:?}", request);
        match request {
            IncomingRequest::Global(r) => r.accept(vec![]),
            IncomingRequest::OpenSession(r) => {
                let session = r.accept();
                //log::warn!("New session: {:?}", session);
            }
            _ => (),
        }
    }*/

    let session = conn.open_session().await??;
    let mut process = session
        .exec("for i in 1 2 3 4 5 6 7 8 9; do echo $i && sleep 1; done".into())
        .await?;
    while let Some(i) = process.next().await {
        log::error!("EVENT {:?}", i);
    }

    //let mut buf: [u8;32] = [0;32];
    //process.read(&mut buf).await?;
    //log::info!("READ STDOUT {:?}", String::from_utf8(Vec::from(&buf[..])));

    async_std::task::sleep(std::time::Duration::from_secs(30)).await;
    log::error!("FOO");
    Ok(())
}

fn main() {
    env_logger::init();

    log::error!(
        "Transport: {}",
        std::mem::size_of::<Transport<Client, TcpStream>>(),
    );
    log::error!(
        "ClientKex: {}",
        std::mem::size_of::<ClientKex>(),
    );
    log::error!(
        "Delay: {}",
        std::mem::size_of::<Delay>(),
    );
    log::error!(
        "ConnectionFuture: {}",
        std::mem::size_of::<ConnectionFuture<Transport<Client, TcpStream>>>()
    );
    log::error!(
        "VecDequeue: {}",
        std::mem::size_of::<VecDeque<oneshot::Receiver<GlobalReply>>>()
    );

    async_std::task::block_on(async move {
        let client = Client::default();
        //client.config().alive_interval = std::time::Duration::from_millis(300);
        match client.connect("localhost:22").await {
            Err(e) => log::error!("{:?}", e),
            Ok(conn) => match foobar(conn).await {
                Ok(()) => log::info!("Allright."),
                Err(e) => log::error!("Exit: {:?}", e),
            },
        }
        async_std::task::sleep(std::time::Duration::from_secs(15)).await;
    })
}
