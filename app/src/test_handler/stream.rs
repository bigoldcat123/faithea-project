use std::time::Duration;

use bytes::Bytes;
use faithea::{
    get, res_modifiers,
    response::{sse::SSE, stream::Stream},
};
use log::info;
use tokio::time::sleep;

#[get("/stream")]
async fn stream() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(23);
    tokio::spawn(async move {
        let mut x = 10;
        while x > 0 {
            tx.send(Bytes::copy_from_slice("data: hello\n".as_bytes()))
                .await
                .unwrap();
            x -= 1;
            info!("??");
            sleep(Duration::from_millis(100)).await;
        }
        info!("ok");
        tx.send(Bytes::copy_from_slice("\n".as_bytes()))
            .await
            .unwrap();
    });

    res_modifiers!(Stream::new(rx), SSE)
}
