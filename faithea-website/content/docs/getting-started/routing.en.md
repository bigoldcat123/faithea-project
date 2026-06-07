---
title: Routing
description: Define HTTP methods, route parameters, and mounted route groups.
---

Routes connect an HTTP method and URL pattern to an async handler. Faithea provides route macros for the most common HTTP methods.

## HTTP methods

Import the method macros you need, then annotate each handler:

```rust
use faithea::{delete, get, handlers, post, put};

#[get("/users")]
async fn list_users() {
    "list users"
}

#[post("/users")]
async fn create_user() {
    "create user"
}

#[put("/users/{id}")]
async fn update_user(id: u64) {
    format!("update user {id}")
}

#[delete("/users/{id}")]
async fn delete_user(id: u64) {
    format!("delete user {id}")
}
```

The same path can use different handlers when its HTTP methods differ.

## Path parameters

Declare a dynamic path segment with braces:

```rust
#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

The route parameter name and handler argument name must match. Faithea converts common parameter types such as `String`, integers, floats, and booleans.

If conversion fails, the request becomes a framework error before the handler runs.

## Multiple parameters

A route can contain more than one parameter:

```rust
#[get("/teams/{team_id}/users/{user_id}")]
async fn team_user(team_id: u64, user_id: u64) {
    format!("team {team_id}, user {user_id}")
}
```

The argument order is flexible, but every route parameter must have a matching handler argument.

## Mount route groups

Collect related handlers and mount them under a common prefix:

```rust
use faithea::{handlers, server::HttpServer};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount(
            "/api",
            handlers!(list_users, create_user, update_user, delete_user),
        )
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

The prefix is combined with each handler route. For example, `#[get("/users")]` becomes `GET /api/users`.

## Try the routes

Start the server and send requests from another terminal:

```sh
curl http://127.0.0.1:3000/api/users
curl -X POST http://127.0.0.1:3000/api/users
curl -X PUT http://127.0.0.1:3000/api/users/42
curl -X DELETE http://127.0.0.1:3000/api/users/42
```

Continue with [Request Data](./request-data.md) to read query parameters, JSON bodies, and request metadata.
