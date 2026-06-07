---
title: 路由
description: 定义 HTTP 方法、路径参数和路由分组。
---

路由负责将 HTTP 方法和 URL 模式连接到异步 handler。Faithea 为常用 HTTP 方法提供了对应的路由宏。

## HTTP 方法

导入需要的方法宏，然后标注每个 handler：

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

当 HTTP 方法不同时，同一个路径可以使用不同的 handler。

## 路径参数

使用大括号声明动态路径片段：

```rust
#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

路径参数名称必须与 handler 参数名称一致。Faithea 可以转换 `String`、整数、浮点数和布尔值等常用参数类型。

如果转换失败，请求会在 handler 运行前变成框架错误。

## 多个路径参数

一个路由可以包含多个参数：

```rust
#[get("/teams/{team_id}/users/{user_id}")]
async fn team_user(team_id: u64, user_id: u64) {
    format!("team {team_id}, user {user_id}")
}
```

参数顺序可以不同，但每个路径参数都必须拥有同名的 handler 参数。

## 挂载路由分组

收集相关 handler，并将它们挂载到共同前缀下：

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

前缀会与每个 handler 路由组合。例如，`#[get("/users")]` 最终会成为 `GET /api/users`。

## 测试路由

启动服务，然后从另一个终端发送请求：

```sh
curl http://127.0.0.1:3000/api/users
curl -X POST http://127.0.0.1:3000/api/users
curl -X PUT http://127.0.0.1:3000/api/users/42
curl -X DELETE http://127.0.0.1:3000/api/users/42
```

继续阅读[请求数据](./request-data.md)，了解如何读取查询参数、JSON 请求体和请求元数据。
