use rssh::client::*;
use rssh::service::connection::*;
use async_std::io::ReadExt;

use env_logger;

async fn foobar(mut conn: Connection<Client>) -> Result<(), ConnectionError> {
    let session = conn.session().await??;
    let mut process = session.exec("/bin/date".into()).await?;
    let mut buf: [u8;32] = [0;32];
    process.read(&mut buf).await?;
    log::info!("READ STDOUT {:?}", String::from_utf8(Vec::from(&buf[..])));

    async_std::task::sleep(std::time::Duration::from_secs(30)).await;    
    conn.disconnect().await;
    Ok(())
}

fn main() {
    env_logger::init();

    futures::executor::block_on(async move {
        let client = Client::default();
        match client.connect("localhost:22").await {
            Err(e) => log::error!("{:?}", e),
            Ok(conn) => {
                match foobar(conn).await {
                    Ok(()) => log::info!("Allright."),
                    Err(e) => log::error!("Exit: {:?}", e),
                }
            },
        }
    })
}
