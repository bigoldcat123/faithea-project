---
title: WebSockets
description: 通过持久连接处理双向消息。
---

在服务构建器上注册 WebSocket handler。Handler 会接收 socket 和原始 `HttpRequest`。

## 定义 Handler

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

## 注册端点

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

连接到 `ws://127.0.0.1:3000/ws/Ada`。请求路径参数仍可以通过 `HttpRequest` 获取。

## 连接生命周期

`split` 会返回接收消息的 receiver 和发送消息的 sender。连接关闭后，接收循环会结束。

对于聊天或广播系统，可以在共享应用状态中存储 sender 的克隆，并在客户端断开时删除它们。
