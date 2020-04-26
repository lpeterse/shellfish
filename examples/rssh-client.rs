use async_std::stream::StreamExt;
use rssh::*;
use std::net::Ipv4Addr;

async fn foobar(mut conn: Connection) -> Result<DisconnectReason, ConnectionError> {
    let stream = conn
        .open_direct_tcpip("localhost", 22, Ipv4Addr::LOCALHOST.into(), 0)
        .await?;

    log::debug!("{:?}", stream);

    /*
    log::warn!("Waiting for requests");

    while let Some(request) = conn.next().await {
        match request {
            ConnectionRequest::Global(r) => {
                log::warn!("Incoming request: {:?}", r);
            }
            ConnectionRequest::ChannelOpen(r) => {
                r.reject(ChannelOpenFailure::UNKNOWN_CHANNEL_TYPE);
            }
        }
    }*/

    log::warn!("Waiting for disconnect or error");

    conn.await
}

fn main() {
    env_logger::init();

    log::error!(
        "ConnectionState {:?}",
        std::mem::size_of::<ConnectionState>()
    );
    //log::error!("ChannelState {:?}", std::mem::size_of::<ChannelState>());
    log::error!(
        "ConnectionError {:?}",
        std::mem::size_of::<Option<ConnectionError>>()
    );

    async_std::task::block_on(async move {
        let client = Client::default();
        // let mut connections = Vec::with_capacity(100);
        // for _ in 1..2 {
        //     match client.connect("localhost:22").await {
        //         Ok(c) => connections.push(c),
        //         Err(e) => log::error!("{:?}", e)
        //     }
        // }
        // log::error!("Setup 1000 connections!");
        match client.connect("localhost:22").await {
            Err(e) => log::error!("{:?}", e),
            Ok(conn) => match foobar(conn).await {
                Ok(d) => log::info!("Disconnect: {:?}", d),
                Err(e) => log::error!("Exit: {:?}", e),
            },
        }
        async_std::task::sleep(std::time::Duration::from_secs(60)).await;
    })
}
