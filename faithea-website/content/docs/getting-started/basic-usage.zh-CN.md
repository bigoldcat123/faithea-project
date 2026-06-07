---
title: 基本用法
description: 构建并运行你的第一个 Faithea HTTP 服务。
---

本指南将构建一个包含两个路由的小型 HTTP 服务。你会体验完整的 Faithea 使用流程：定义 handler、挂载路由、启动服务并发送请求。

## 创建第一个服务

使用以下代码替换 `src/main.rs`：

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

## 定义 Handler

`#[get]` 宏会将异步函数转换为处理 GET 路由的 handler：

```rust
#[get("/")]
async fn index() {
    "Hello, Faithea!"
}
```

Handler 函数不需要显式声明返回类型。Faithea 会将字符串等受支持的值转换为 HTTP 响应。

路径参数使用大括号声明，并通过相同名称传入 handler：

```rust
#[get("/hello/{name}")]
async fn hello(name: String) {
    format!("Hello, {name}!")
}
```

访问 `/hello/Ada` 时，Faithea 会提取 `Ada`，并将它传给 `name` 参数。

## 挂载路由

`handlers!` 宏负责收集 handler，`mount` 则将它们挂载到共同的路径前缀下：

```rust
.mount("/", handlers!(index, hello))
```

挂载到 `/` 时，两个路由分别保持为 `/` 和 `/hello/{name}`。如果使用 `/api` 前缀，它们将变为 `/api` 和 `/api/hello/{name}`。

## 启动服务

启动应用：

```sh
cargo run
```

示例明确监听 `127.0.0.1:3000`。保持服务运行，然后从另一个终端发送请求。

## 发送请求

访问首页路由：

```sh
curl http://127.0.0.1:3000/
```

```text
Hello, Faithea!
```

然后尝试路径参数：

```sh
curl http://127.0.0.1:3000/hello/Ada
```

```text
Hello, Ada!
```

现在你已经拥有一个可以工作的 Faithea 服务。接下来的指南可以在此基础上继续介绍请求数据、JSON 响应和更多路由类型。
