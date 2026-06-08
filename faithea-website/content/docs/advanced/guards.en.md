---
title: Guards
description: Inspect, allow, or reject requests before handlers run.
---

Guards run before route handlers. They can inspect a request, pass it to the next guard, or stop processing with an HTTP response.

## Add a guard

Register guards on the server builder:

```rust
use faithea::{
    get, handlers,
    response::HttpResponse,
    server::HttpServer,
};

#[get("/dashboard")]
async fn dashboard() {
    "protected dashboard"
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(dashboard))
        .guard("/dashboard", async |req| {
            match req.get_header("authorization") {
                Some(value) if value == "Bearer secret" => Ok(req),
                _ => Err(HttpResponse::error("unauthorized".into())),
            }
        })
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

Returning `Ok(req)` continues processing. Returning `Err(response)` sends that response immediately and skips the handler.

## Guard route patterns

Guards support the same route patterns as handlers:

```rust
.guard("/api/**", async |req| {
    println!("request: {}", req.uri());
    Ok(req)
})
```

Use an exact path for one endpoint or `/**` to cover a route group.

## Guard chains

Multiple matching guards run as a chain. Each guard receives the request returned by the previous guard. Any guard can stop the chain by returning a response.

## Test the guard

First send a request without the authentication header. The guard rejects it:

```sh
curl -i http://127.0.0.1:3000/dashboard
```

Then include the correct Authorization header. The request continues to the handler:

```sh
curl -i http://127.0.0.1:3000/dashboard \
  -H "authorization: Bearer secret"
```

Keep guards focused on cross-cutting request checks such as authentication, authorization, logging, or request policy.
