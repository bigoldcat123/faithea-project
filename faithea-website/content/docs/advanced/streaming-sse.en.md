---
title: Streaming & SSE
description: Stream chunks over time and publish server-sent events.
---

`Stream` sends byte chunks from a Tokio channel. Combine it with `SSE` to produce a server-sent event response.

## Add dependencies

```sh
cargo add bytes
cargo add tokio --features sync,time
```

## Create an SSE endpoint

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

SSE messages end with a blank line. `SSE` adds the `text/event-stream` content type and disables caching.

## Test the stream

```sh
curl -N http://127.0.0.1:3000/events
```

Use a bounded channel to apply backpressure. Stop producing data when `send` fails, because that usually means the client disconnected.
