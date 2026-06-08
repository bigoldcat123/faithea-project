---
title: 请求守卫
description: 在 handler 运行前检查、放行或拒绝请求。
---

Guard 会在路由 handler 之前运行。它可以检查请求、将请求传递给下一个 Guard，或通过 HTTP 响应停止处理。

## 添加 Guard

在服务构建器上注册 Guard：

```rust
use faithea::{
    get, handlers,
    response::HttpResponse,
    server::HttpServer,
};

#[get("/dashboard")]
async fn dashboard() {
    "protected dashboard"
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(dashboard))
        .guard("/dashboard", async |req| {
            match req.get_header("authorization") {
                Some(value) if value == "Bearer secret" => Ok(req),
                _ => Err(HttpResponse::error("unauthorized".into())),
            }
        })
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

返回 `Ok(req)` 会继续处理。返回 `Err(response)` 会立即发送响应并跳过 handler。

## Guard 路由模式

Guard 支持与 handler 相同的路由模式：

```rust
.guard("/api/**", async |req| {
    println!("request: {}", req.uri());
    Ok(req)
})
```

可以使用精确路径保护单个端点，也可以使用 `/**` 覆盖整个路由分组。

## Guard 链

多个匹配的 Guard 会组成一条链。每个 Guard 都会接收上一个 Guard 返回的请求，任意 Guard 都可以通过返回响应停止链。

## 测试 Guard

先发送一个没有认证 Header 的请求，它会被 Guard 拒绝：

```sh
curl -i http://127.0.0.1:3000/dashboard
```

再带上正确的 Authorization Header，请求会继续进入 handler：

```sh
curl -i http://127.0.0.1:3000/dashboard \
  -H "authorization: Bearer secret"
```

Guard 适合处理认证、授权、日志和请求策略等跨路由检查。
