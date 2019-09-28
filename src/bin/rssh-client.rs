use rssh::client::*;
use rssh::service::connection::*;
use std::future::Future;
use env_logger;
use log::*;


async fn foobar(mut conn: Connection) -> Result<(), ConnectionError> {
    log::error!("CONNECTED");

    async_std::task::sleep(std::time::Duration::from_secs(5)).await;
    let session = conn.session().await??;
    log::info!("SESSION OPEN");

    async_std::task::sleep(std::time::Duration::from_secs(10)).await;
    let process = session.exec("/bin/date".into()).await?;
    log::info!("PROCESS STARTED");

    async_std::task::sleep(std::time::Duration::from_secs(10)).await;    
    conn.disconnect().await;
    println!("DISCONNE");
    Ok(())
}

fn main() {
    env_logger::init();
    info!("shdksjhda");

    futures::executor::block_on(async move {
        let mut client = Client::default();
        //*client.user_name() = None;
        let conn = client
            .connect("localhost:22")
            .await;
        log::error!("OOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOO");
        match conn {
            Err(e) => println!("{:?}", e),
            Ok(mut conn) => {
                match foobar(conn).await {
                    Ok(()) => log::info!("Allright."),
                    Err(e) => log::error!("Exit: {:?}", e),
                }
            },
        }
    })
}
