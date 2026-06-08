---
title: 高级响应
description: 构建重定向、Cookie 和可复用的自定义响应修改器。
---

Faithea 响应由实现 `HttpResponseModifier` 的类型组装而成。你可以组合内置修改器，也可以定义自己的修改器。

## 重定向响应

返回 `Redirect` 发送永久重定向。下面的例子会把 `/old-page` 重定向到 `/new-page`。

### 定义路由

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/old-page")]
async fn old_page() {
    Redirect("/new-page")
}

#[get("/new-page")]
async fn new_page() {
    "new page"
}
```

### 测试 curl

使用 `-i` 查看重定向状态码和 `location` Header：

```sh
curl -i http://127.0.0.1:3000/old-page
```

使用 `-L` 让 curl 跟随重定向：

```sh
curl -i -L http://127.0.0.1:3000/old-page
```

## 设置 Cookie

Cookie 本质上就是响应里的 `set-cookie` Header。可以直接构建 `HeaderMap`，设置 `SET_COOKIE`，再将 Header 与响应体组合。

随后可以在 handler 中使用 `_req.cookies()` 检查请求 Cookie。

### 定义路由

```rust
use faithea::{
    HeaderMap,
    get,
    header::{HeaderValue, SET_COOKIE},
    res_modifiers,
};

#[get("/login")]
async fn login() {
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_static("session=abc123; HttpOnly; Path=/"),
    );

    res_modifiers!(headers, "logged in")
}

#[get("/profile")]
async fn profile() {
    format!("{:?}", _req.cookies())
}
```

### 测试 curl

先请求登录接口，并把响应 Cookie 保存到 `cookies.txt`：

```sh
curl -i -c cookies.txt http://127.0.0.1:3000/login
```

再携带保存的 Cookie 访问需要会话信息的接口：

```sh
curl -i -b cookies.txt http://127.0.0.1:3000/profile
```

## 创建响应修改器

上一节我们用 `YamlBody<T>` 解析 YAML 请求体。现在给同一个包装类型实现 `HttpResponseModifier`，让 handler 也可以直接返回 YAML 响应。

先添加依赖：

```sh
cargo add bytes
cargo add serde --features derive
cargo add serde_yaml
```

### 定义响应修改器

`HttpResponseModifier` 可以修改响应 Header、状态码和响应体。这里我们将内部值序列化为 YAML，设置 `content-type` 与 `content-length`，再写入响应体。

```rust
use bytes::Bytes;
use faithea::{
    get,
    handler::types::HttpHandlerError,
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture, ResponseBody},
};
use serde::Serialize;

struct YamlBody<T>(T);

impl<T: Serialize + Send + Sync> HttpResponseModifier for YamlBody<T> {
    fn modify<'a>(&'a mut self, res: &'a mut HttpResponse) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            let body = serde_yaml::to_string(&self.0)
                .map_err(|_| HttpHandlerError::after_handler_incompatible_body_type())?;

            res.add_header("content-type", "application/x-yaml".parse()?);
            res.add_header("content-length", body.len().to_string().parse()?);
            res.set_body(ResponseBody::Simple(Bytes::from(body)));

            Ok(())
        })
    }
}
```

修改器会按顺序应用。自定义修改器应只更新自己负责的响应属性。

### 使用响应修改器

```rust
#[derive(Serialize)]
struct DeployConfig {
    service: String,
    replicas: u8,
    public: bool,
}

#[get("/deploy-config.yaml")]
async fn deploy_config_yaml() {
    YamlBody(DeployConfig {
        service: "api".into(),
        replicas: 3,
        public: true,
    })
}
```

### 测试 curl

```sh
curl -i http://127.0.0.1:3000/deploy-config.yaml
```

响应体会是 YAML，响应 Header 会包含 `content-type: application/x-yaml`。
