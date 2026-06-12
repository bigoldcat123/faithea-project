use std::error::Error;

use bytes::{BufMut, BytesMut};
use faithea_websocket::{
    WebSocketDataPayLoad, WebSocketIncommingMessageParser, WebSocketMessageType,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    sync::mpsc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let (client, server) = tokio::io::duplex(1024);
    let (server_reader, mut server_writer) = tokio::io::split(server);
    let (mut client_reader, mut client_writer) = tokio::io::split(client);

    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<WebSocketDataPayLoad>(16);
    let app_outgoing_tx = outgoing_tx.clone();
    let (parser, mut incoming_rx) =
        WebSocketIncommingMessageParser::new(server_reader, outgoing_tx);
    parser.start();

    let writer_task = tokio::spawn(async move {
        while let Some(message) = outgoing_rx.recv().await {
            if message
                .serialize_to_socket(&mut server_writer)
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let app_task = tokio::spawn(async move {
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
    });

    client_writer
        .write_all(&masked_client_frame(WebSocketMessageType::Text, b"hello"))
        .await?;

    let echoed = read_unmasked_server_frame(&mut client_reader).await?;
    println!("echoed text: {}", String::from_utf8(echoed)?);

    client_writer
        .write_all(&masked_client_frame(WebSocketMessageType::Close, b""))
        .await?;
    let _ = read_unmasked_server_frame(&mut client_reader).await?;

    app_task.await?;
    writer_task.await?;
    Ok(())
}

fn masked_client_frame(opcode: WebSocketMessageType, payload: &[u8]) -> Vec<u8> {
    let mask = [1, 2, 3, 4];
    let mut frame = BytesMut::new();
    frame.put_u8(0x80 | u8::from(opcode));

    if payload.len() < 126 {
        frame.put_u8(0x80 | payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.put_u8(0x80 | 126);
        frame.put_u16(payload.len() as u16);
    } else {
        frame.put_u8(0x80 | 127);
        frame.put_u64(payload.len() as u64);
    }

    frame.put_slice(&mask);
    for (index, byte) in payload.iter().enumerate() {
        frame.put_u8(byte ^ mask[index % mask.len()]);
    }
    frame.to_vec()
}

async fn read_unmasked_server_frame<R>(reader: &mut R) -> Result<Vec<u8>, Box<dyn Error>>
where
    R: AsyncRead + Unpin,
{
    let mut head = [0u8; 2];
    reader.read_exact(&mut head).await?;

    let len = match head[1] & 0x7f {
        126 => {
            let mut extended = [0u8; 2];
            reader.read_exact(&mut extended).await?;
            u16::from_be_bytes(extended) as usize
        }
        127 => {
            let mut extended = [0u8; 8];
            reader.read_exact(&mut extended).await?;
            usize::try_from(u64::from_be_bytes(extended))?
        }
        len => len as usize,
    };

    let mut payload = vec![0; len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}
