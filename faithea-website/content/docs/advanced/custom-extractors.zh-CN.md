---
title: 自定义请求提取器
description: 将 HttpRequest 转换为可复用的类型化 handler 参数。
---

自定义提取器可以将重复的请求解析逻辑移出 handler。实现 `TryFromRequest` 后，即可通过 `FromRequest<T>` 接收提取结果。

## 定义提取器

以下提取器会读取 Authorization Header：

```rust
use faithea::{
    data::{Json, inbound::FromRequest},
    get,
    handler::types::HttpHandlerError,
    request::{HttpRequest, TryFromRequest},
};
use serde::Serialize;

#[derive(Serialize)]
struct CurrentUser {
    token: String,
}

impl<'a> TryFromRequest<'a> for CurrentUser {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        let token = req
            .get_header("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(HttpHandlerError::before_handler_param_not_exist)?;

        Ok(CurrentUser {
            token: token.to_string(),
        })
    }
}
```

## 使用提取器

使用 `FromRequest<T>` 包装自定义类型：

```rust
#[get("/me")]
async fn me(user: FromRequest<CurrentUser>) {
    Json(user.into_inner())
}
```

Faithea 会在 handler 前调用 `TryFromRequest`。提取失败时，错误会进入普通的全局错误处理流程。

## 合适的提取职责

自定义提取器适合封装当前登录用户、请求 ID、经过验证的 Header 和其他类型化请求上下文。

提取器应当保持确定性和低开销。耗时的业务操作应在提取成功后交给 service 层。
