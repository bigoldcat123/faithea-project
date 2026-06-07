---
title: 错误处理
description: 理解框架错误，并提供一致的全局错误响应。
---

Faithea 可能在 handler 运行前拒绝请求，也可能在构建响应时产生错误。全局错误处理器可以让这些失败使用一致的响应格式。

## 框架错误

常见框架错误包括：

- 必填路径参数或查询参数缺失
- 参数无法转换为声明的 Rust 类型
- JSON 请求体缺失或无效
- 响应修改器无法构建最终响应

例如，以下路由要求一个整数参数：

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

请求 `/users/not-a-number` 时，参数提取会失败，`get_user` 不会运行。

## 添加全局错误处理器

在服务构建器上使用 `globale_error_handler`，将框架错误转换为统一响应：

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

添加示例使用的依赖：

```sh
cargo add serde --features derive
cargo add http
```

回调会接收 `faithea::error::Error`，并返回普通响应修改器。因此，成功请求和框架错误可以使用同一种响应模型。

## 测试无效请求

启动服务，然后提供无效整数：

```sh
curl -i http://127.0.0.1:3000/users/not-a-number
```

响应会使用 `400 Bad Request` 状态码，并通过 JSON 响应体描述框架错误。

## 制定错误策略

为了保持示例简单，这里将所有框架错误映射为 `400 Bad Request`。生产应用应当谨慎选择状态码和公开错误信息，并避免暴露敏感内部细节。

来自 service 层的业务错误，也应当在 handler 边界转换为一致的公开响应。
