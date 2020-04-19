use async_std::stream::StreamExt;
use rssh::client::*;
use rssh::service::connection::*;

async fn foobar(mut conn: Connection) -> Result<(), ConnectionError> {

    match conn.open_session().await? {
        Ok(session) => {
            log::warn!("SESSION CONFIRMED: {:?}", session);
        }
        Err(reason) => {
            log::warn!("SESSION FAILURE: {:?}", reason);
        }
    }

    log::warn!("Waiting for requests");

    while let Some(request) = conn.next().await {
        match request? {
            ConnectionRequest::Global(r) => {
                log::warn!("Incoming request: {:?}", r);
            }
            ConnectionRequest::ChannelOpen(r) => {
                r.reject(ChannelOpenFailureReason::UNKNOWN_CHANNEL_TYPE);
            }
        }
    }

    Ok(())
}

fn main() {
    env_logger::init();

    async_std::task::block_on(async move {
        let client = Client::default();
        match client.connect("localhost:22").await {
            Err(e) => log::error!("{:?}", e),
            Ok(conn) => match foobar(conn).await {
                Ok(()) => log::info!("Allright."),
                Err(e) => log::error!("Exit: {:?}", e),
            },
        }
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
    })
}
