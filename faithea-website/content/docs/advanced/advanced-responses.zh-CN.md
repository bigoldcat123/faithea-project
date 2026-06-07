---
title: 高级响应
description: 构建重定向、Cookie 和可复用的自定义响应修改器。
---

Faithea 响应由实现 `HttpResponseModifier` 的类型组装而成。你可以组合内置修改器，也可以定义自己的修改器。

## 重定向响应

返回 `Redirect` 发送永久重定向：

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/old-page")]
async fn old_page() {
    Redirect("/new-page")
}
```

## 设置 Cookie

构建响应 Cookie，并将它与响应体组合：

```rust
use faithea::{
    get, res_modifiers,
    response::cookie::{Cookie, CookieType},
};

#[get("/login")]
async fn login() {
    let mut cookie = Cookie::default();
    cookie.push(CookieType::KeyValue("session".into(), "abc123".into()));
    cookie.push(CookieType::Attribute("HttpOnly".into()));

    res_modifiers!(cookie, "logged in")
}
```

在 handler 中使用 `_req.cookies()` 可以检查请求 Cookie。

## 创建响应修改器

为可复用响应行为实现 `HttpResponseModifier`：

```rust
use faithea::{
    get,
    header::HeaderValue,
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture},
};

struct RequestId(&'static str);

impl HttpResponseModifier for RequestId {
    fn modify<'a>(&'a mut self, res: &'a mut HttpResponse) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header("x-request-id", HeaderValue::from_static(self.0));
            Ok(())
        })
    }
}

#[get("/custom")]
async fn custom() {
    faithea::res_modifiers!(RequestId("req-42"), "custom response")
}
```

修改器会按顺序应用。自定义修改器应只更新自己负责的响应属性。
