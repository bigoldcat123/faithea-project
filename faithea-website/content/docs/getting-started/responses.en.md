---
title: Responses
description: Return text, JSON, status codes, and headers from handlers.
---

A Faithea handler returns one or more response modifiers. Each modifier changes part of the outgoing HTTP response.

## Text responses

Return `&str` or `String` for a plain-text response:

```rust
use faithea::get;

#[get("/health")]
async fn health() {
    "ok"
}

#[get("/greet/{name}")]
async fn greet(name: String) {
    format!("Hello, {name}!")
}
```

Handler functions do not need an explicit return type. The route macro supplies the response modifier type.

## JSON responses

Values wrapped in `Json<T>` are serialized as JSON. The inner value must implement `Serialize`:

```rust
use faithea::{data::Json, get};
use serde::Serialize;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

#[get("/health.json")]
async fn health_json() {
    Json(Health { status: "ok" })
}
```

Add Serde when your project does not already use it:

```sh
cargo add serde --features derive
```

## Combine response modifiers

Use `res_modifiers!` when one handler needs to change several response properties:

```rust
use faithea::{
    HeaderMap,
    get,
    header::HeaderValue,
    res_modifiers,
};
use http::StatusCode;

#[get("/created")]
async fn created() {
    let mut headers = HeaderMap::new();
    headers.insert("x-faithea-example", HeaderValue::from_static("created"));

    res_modifiers!(
        StatusCode::CREATED,
        headers,
        "resource created",
    )
}
```

This example needs the `http` crate for `StatusCode`:

```sh
cargo add http
```

The modifiers are applied in order. Here they set the status code, add a custom header, and write the response body.

## Supported response concepts

The same response-modifier model is used throughout Faithea:

| Modifier | Purpose |
| --- | --- |
| `&str` and `String` | Plain-text body |
| `Json<T>` | JSON body and content type |
| `StatusCode` | HTTP status |
| `HeaderMap` | Response headers |
| `res_modifiers!(...)` | Combine multiple modifiers |

More specialized modifiers, such as redirects, files, streams, and CORS, belong in the advanced guides.

Continue with [Error Handling](./error-handling.md) to customize failures produced while processing requests.
