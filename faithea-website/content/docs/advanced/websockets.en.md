---
title: WebSockets
description: Handle bidirectional messages over a persistent connection.
---

Register a WebSocket handler on the server builder. The handler receives the socket and its original `HttpRequest`.

## Define a handler

```rust
use faithea::{
    request::HttpRequest,
    websocket::{data::WebSocketDataPayLoad, socket::WebSocket},
};

async fn echo(websocket: WebSocket, req: HttpRequest) {
    let name = req
        .get_pathparam("name")
        .cloned()
        .unwrap_or_else(|| "anonymous".into());
    let (mut receiver, sender) = websocket.split();

    while let Some(message) = receiver.recv().await {
        let text = String::from_utf8_lossy(message.as_bytes());
        if sender
            .send(WebSocketDataPayLoad::text(format!("{name}: {text}")))
            .await
            .is_err()
        {
            break;
        }
    }
}
```

## Register the endpoint

```rust
use faithea::server::HttpServer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .websocket("/ws/{name}", echo)
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

Connect to `ws://127.0.0.1:3000/ws/Ada`. The request path parameter remains available through `HttpRequest`.

## Connection lifecycle

`split` returns a receiver for incoming messages and a sender for outgoing messages. The receive loop ends when the connection closes.

For chat or broadcast systems, store cloned senders in shared application state and remove them when clients disconnect.
