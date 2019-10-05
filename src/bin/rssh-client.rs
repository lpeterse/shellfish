use rssh::client::*;
use rssh::service::connection::*;
use futures::io::AsyncReadExt;

use env_logger;

async fn foobar(mut conn: Connection<Client>) -> Result<(), ConnectionError> {
    log::error!("CONNECTED");

    //async_std::task::sleep(std::time::Duration::from_secs(5)).await;
    let session = conn.session().await??;
    log::info!("SESSION OPEN");

    //async_std::task::sleep(std::time::Duration::from_secs(10)).await;
    let mut process = session.exec("/bin/date".into()).await?;
    log::info!("BLOB");
    let mut buf: [u8;32] = [0;32];
    process.read(&mut buf).await?;
    log::error!("READ STDOUT {:?}", String::from_utf8(Vec::from(&buf[..])));

    async_std::task::sleep(std::time::Duration::from_secs(30)).await;    
    conn.disconnect().await;
    println!("DISCONNE");
    Ok(())
}

fn main() {
    env_logger::init();

    futures::executor::block_on(async move {
        let client = Client::default();
        match client.connect("localhost:22").await {
            Err(e) => println!("{:?}", e),
            Ok(conn) => {
                match foobar(conn).await {
                    Ok(()) => log::info!("Allright."),
                    Err(e) => log::error!("Exit: {:?}", e),
                }
            },
        }
    })
}
