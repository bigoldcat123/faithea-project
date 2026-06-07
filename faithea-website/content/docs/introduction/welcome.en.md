---
title: Welcome
description: A short introduction to Faithea and its guiding ideas.
---

Faithea is a lightweight asynchronous HTTP framework for Rust, built on Tokio.

It focuses on a small, understandable API while keeping the important parts of HTTP development explicit. Routes are declared with expressive macros, request data is extracted into Rust types, and responses remain composable.

## Why Faithea

Faithea is designed for developers who want to:

- Build asynchronous HTTP services without unnecessary machinery.
- Keep routing and handler code compact and readable.
- Extend request and response behavior with ordinary Rust types.
- Stay close to Rust and Tokio instead of hiding them.

## A tiny example

```rust
use faithea::{get, HttpServer};

#[get("/hello/{name}")]
async fn hello(name: String) {
    format!("Hello, {name}!")
}
```

This documentation will grow alongside the framework. The next section, **Getting Started**, will walk through creating and running a small service.
