use rssh::client::*;
use rssh::service::connection::*;

use async_std::stream::StreamExt;

async fn foobar(mut conn: Connection) -> Result<(), ConnectionError> {
    let session = conn.session().await??;
    let mut process = session.exec("for i in 1 2 3 4 5 6 7 8 9; do echo $i && sleep 1; done".into()).await?;
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
