---
title: 响应
description: 从 handler 返回文本、JSON、状态码和 Header。
---

Faithea handler 会返回一个或多个响应修改器。每个修改器负责改变 HTTP 响应的一部分。

## 文本响应

返回 `&str` 或 `String`，即可生成纯文本响应：

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

Handler 函数不需要显式声明返回类型。路由宏会补充响应修改器类型。

## JSON 响应

使用 `Json<T>` 包装的值会被序列化为 JSON。内部值必须实现 `Serialize`：

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

如果项目还没有使用 Serde，请添加依赖：

```sh
cargo add serde --features derive
```

## 组合响应修改器

当一个 handler 需要修改多个响应属性时，使用 `res_modifiers!`：

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

这个示例需要通过 `http` crate 使用 `StatusCode`：

```sh
cargo add http
```

修改器会按顺序应用。这里依次设置状态码、添加自定义 Header，并写入响应体。

## 支持的响应概念

Faithea 的其他响应能力也使用相同的响应修改器模型：

| 修改器 | 用途 |
| --- | --- |
| `&str` 和 `String` | 纯文本响应体 |
| `Json<T>` | JSON 响应体与内容类型 |
| `StatusCode` | HTTP 状态码 |
| `HeaderMap` | 响应 Header |
| `res_modifiers!(...)` | 组合多个修改器 |

重定向、文件、流和 CORS 等专用修改器更适合在高级指南中介绍。

继续阅读[错误处理](./error-handling.md)，了解如何自定义请求处理过程中产生的失败响应。
