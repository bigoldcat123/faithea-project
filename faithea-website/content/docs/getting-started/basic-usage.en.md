---
title: Basic Usage
description: Build and run your first Faithea HTTP server.
---

This guide builds a small HTTP server with two routes. You will see the complete Faithea workflow: define handlers, mount them, start the server, and send requests.

## Create your first server

Replace `src/main.rs` with the following code:

```rust
use faithea::{get, handlers, server::HttpServer};

#[get("/")]
async fn index() {
    "Hello, Faithea!"
}

#[get("/hello/{name}")]
async fn hello(name: String) {
    format!("Hello, {name}!")
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(index, hello))
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

## Define handlers

The `#[get]` macro turns an async function into a handler for a GET route:

```rust
#[get("/")]
async fn index() {
    "Hello, Faithea!"
}
```

Handler functions do not need an explicit return type. Faithea converts supported values, such as strings, into HTTP responses.

Route parameters are declared inside braces and passed into the handler using the same name:

```rust
#[get("/hello/{name}")]
async fn hello(name: String) {
    format!("Hello, {name}!")
}
```

For `/hello/Ada`, Faithea extracts `Ada` and passes it to `name`.

## Mount the routes

The `handlers!` macro collects handlers, and `mount` attaches them under a shared prefix:

```rust
.mount("/", handlers!(index, hello))
```

Mounting at `/` keeps the routes at `/` and `/hello/{name}`. A prefix such as `/api` would expose them at `/api` and `/api/hello/{name}`.

## Run the server

Start the application:

```sh
cargo run
```

The example explicitly listens on `127.0.0.1:3000`. Keep the process running while you send requests from another terminal.

## Send requests

Call the index route:

```sh
curl http://127.0.0.1:3000/
```

```text
Hello, Faithea!
```

Then try the route parameter:

```sh
curl http://127.0.0.1:3000/hello/Ada
```

```text
Hello, Ada!
```

You now have a working Faithea server. The next guides can build on this foundation with request data, JSON responses, and more route types.
