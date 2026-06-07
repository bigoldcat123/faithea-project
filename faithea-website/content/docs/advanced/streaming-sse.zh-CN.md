---
title: 流式响应与 SSE
description: 随时间发送数据块，并发布服务器推送事件。
---

`Stream` 会发送 Tokio channel 中的字节块。将它与 `SSE` 组合，即可生成服务器推送事件响应。

## 添加依赖

```sh
cargo add bytes
cargo add tokio --features sync,time
```

## 创建 SSE 端点

```rust
use std::time::Duration;

use bytes::Bytes;
use faithea::{
    get, res_modifiers,
    response::{sse::SSE, stream::Stream},
};
use tokio::time::sleep;

#[get("/events")]
async fn events() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(16);

    tokio::spawn(async move {
        for number in 1..=5 {
            let event = format!("data: event {number}\n\n");
            if tx.send(Bytes::from(event)).await.is_err() {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
    });

    res_modifiers!(Stream::new(rx), SSE)
}
```

SSE 消息以空行结束。`SSE` 会添加 `text/event-stream` 内容类型并禁用缓存。

## 测试数据流

```sh
curl -N http://127.0.0.1:3000/events
```

使用有界 channel 提供背压。当 `send` 失败时应停止生产数据，因为这通常表示客户端已经断开。
