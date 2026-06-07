---
title: Static Files
description: Return individual files or map an entire directory.
---

Faithea can return one file from a handler or map a URL pattern to a local directory.

## Return one file

Use `StaticFile` as the handler response:

```rust
use faithea::{data::outbound::StaticFile, get};

#[get("/download")]
async fn download() {
    StaticFile("./public/manual.pdf")
}
```

Faithea reads the file asynchronously, sets its content length, and chooses a content type from the extension.

## Map a directory

Use `static_map` on the server builder:

```rust
use faithea::server::HttpServer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .static_map("/assets/**", "./public")
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

A request such as `/assets/icons/logo.svg` is resolved inside `./public`.

## Deployment safety

Map only directories intended for public access. Keep secrets, source files, and generated private data outside the mapped directory.

Use a stable path available in the deployment environment, and verify missing-file behavior through your global error handler.
