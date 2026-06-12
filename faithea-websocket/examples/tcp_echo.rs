use std::error::Error;

use faithea_websocket::{
    WebSocketDataPayLoad, WebSocketIncommingMessageParser, WebSocketMessageType,
};
use tokio::{net::TcpListener, sync::mpsc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:9001").await?;
    println!("listening on 127.0.0.1:9001");
    println!("this example expects an already-upgraded WebSocket byte stream");

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("accepted {peer}");

        tokio::spawn(async move {
            let (reader, mut writer) = tokio::io::split(stream);
            let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<WebSocketDataPayLoad>(32);
            let app_outgoing_tx = outgoing_tx.clone();

            let (parser, mut incoming_rx) =
                WebSocketIncommingMessageParser::new(reader, outgoing_tx);
            parser.start();

            let writer_task = tokio::spawn(async move {
                while let Some(message) = outgoing_rx.recv().await {
                    if message.serialize_to_socket(&mut writer).await.is_err() {
                        break;
                    }
                }
            });

            while let Some(message) = incoming_rx.recv().await {
                match message.message_type() {
                    WebSocketMessageType::Text | WebSocketMessageType::Binary => {
                        if app_outgoing_tx.send(message).await.is_err() {
                            break;
                        }
                    }
                    WebSocketMessageType::Close => break,
                    WebSocketMessageType::Ping
                    | WebSocketMessageType::Pong
                    | WebSocketMessageType::Continuation => {}
                }
            }

            drop(app_outgoing_tx);
            let _ = writer_task.await;
        });
    }
}
