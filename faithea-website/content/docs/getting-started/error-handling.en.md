---
title: Error Handling
description: Understand framework errors and provide a consistent global error response.
---

Faithea can reject a request before the handler runs or while it builds the response. A global error handler gives these failures a consistent response format.

## Framework errors

Common framework errors include:

- A required path or query parameter is missing
- A parameter cannot be converted to its declared Rust type
- A JSON body is missing or invalid
- A response modifier cannot build the outgoing response

For example, this route expects an integer:

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

Requesting `/users/not-a-number` fails during parameter extraction, before `get_user` runs.

## Add a global error handler

Use `globale_error_handler` on the server builder to transform framework errors into a shared response:

```rust
use faithea::{
    data::Json,
    get,
    handlers,
    res_modifiers,
    server::HttpServer,
};
use http::StatusCode;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(get_user))
        .globale_error_handler(async |error: faithea::error::Error| {
            res_modifiers!(
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: format!("{error:?}"),
                }),
            )
        })
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

Add the dependencies used by the example:

```sh
cargo add serde --features derive
cargo add http
```

The callback receives `faithea::error::Error` and returns normal response modifiers. This makes the same response model available for both successful requests and framework failures.

## Test an invalid request

Start the server, then provide an invalid integer:

```sh
curl -i http://127.0.0.1:3000/users/not-a-number
```

The response uses status `400 Bad Request` and a JSON body describing the framework error.

## Choose an error policy

The example maps every framework error to `400 Bad Request` for simplicity. A production application should choose status codes and public messages deliberately, and avoid exposing sensitive internal details.

Business errors from your service layer should also be converted into a consistent public response at the handler boundary.
