---
title: Routing
description: Define HTTP methods, route parameters, and mounted route groups.
---

Routes connect an HTTP method and URL pattern to an async handler. Faithea provides route macros for the most common HTTP methods.

## HTTP methods

The same path can use different handlers when its HTTP methods differ. Import the method macros you need, then annotate each handler.

### Define routes

```rust
use faithea::{delete, get, post, put};

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

### Mount routes

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

### Test routes

```sh
curl http://127.0.0.1:3000/api/users
curl -X POST http://127.0.0.1:3000/api/users
curl -X PUT http://127.0.0.1:3000/api/users/42
curl -X DELETE http://127.0.0.1:3000/api/users/42
```

## Path parameters

Declare a dynamic path segment with braces. The route parameter name and handler argument name must match. Faithea converts common parameter types such as `String`, integers, floats, and booleans.

If conversion fails, the request becomes a framework error before the handler runs.

### Define routes

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

### Mount routes

```rust
use faithea::{handlers, server::HttpServer};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/api", handlers!(get_user))
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

### Test routes

```sh
curl http://127.0.0.1:3000/api/users/42
```

## Multiple path parameters

A route can contain more than one parameter. The argument order is flexible, but every route parameter must have a matching handler argument.

### Define routes

```rust
use faithea::get;

#[get("/teams/{team_id}/users/{user_id}")]
async fn team_user(team_id: u64, user_id: u64) {
    format!("team {team_id}, user {user_id}")
}
```

### Mount routes

```rust
use faithea::{handlers, server::HttpServer};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/api", handlers!(team_user))
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

### Test routes

```sh
curl http://127.0.0.1:3000/api/teams/7/users/42
```

## Wildcard patterns

Use `*` to match one path segment and `**` to match the remaining path.

`/files/*` matches `/files/readme`, while `/assets/**` also matches deeply nested paths such as `/assets/icons/logo.svg`.

Wildcards combine with mounted prefixes. Mounting `nested_assets` at `/api` exposes it at `/api/assets/**`.

### Define routes

```rust
use faithea::get;

#[get("/files/*")]
async fn one_level() {
    format!("one level: {}", _req.uri())
}

#[get("/assets/**")]
async fn nested_assets() {
    format!("nested asset: {}", _req.uri())
}
```

### Mount routes

```rust
use faithea::{handlers, server::HttpServer};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/api", handlers!(one_level, nested_assets))
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

### Test routes

```sh
curl http://127.0.0.1:3000/api/files/readme
curl http://127.0.0.1:3000/api/assets/icons/logo.svg
```

### Route precedence

When several patterns match, Faithea prefers more specific routes:

```text
exact segment > path parameter > * > **
```

Define an exact route for special cases and keep a wildcard as the fallback. Use broad `/**` routes deliberately, because they can match requests that would otherwise become `404 Not Found`.

Continue with [Request Data](./request-data.md) to read query parameters, JSON bodies, and request metadata.
