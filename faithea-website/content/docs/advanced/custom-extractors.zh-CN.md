---
title: 自定义请求提取器
description: 将 HttpRequest 转换为可复用的类型化 handler 参数。
---

自定义提取器可以将重复的请求解析逻辑移出 handler。实现 `TryFromRequest` 后，即可通过 `FromRequest<T>` 接收提取结果。

Faithea 会在 handler 前调用 `TryFromRequest`。提取失败时，错误会进入普通的全局错误处理流程。

## Token 解析

这个例子会从 Authorization Header 中读取 token，并将它包装成 `CurrentUser`。

### 定义提取器

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

### 使用提取器

使用 `FromRequest<T>` 包装自定义类型：

```rust
#[get("/me")]
async fn me(user: FromRequest<CurrentUser>) {
    Json(user.into_inner())
}
```

### 测试提取器

先发送一个没有 Authorization Header 的请求，它会在进入 handler 前失败：

```sh
curl -i http://127.0.0.1:3000/me
```

再带上 Authorization Header，请求会被提取为 `CurrentUser` 并返回 JSON：

```sh
curl -i http://127.0.0.1:3000/me \
  -H "authorization: Bearer secret"
```

## YAML Body 解析

自定义提取器也可以解析请求体。下面我们一起创建一个 `YamlBody<T>`，让 handler 可以像使用 `Json<T>` 一样接收 YAML 请求体。

先添加依赖：

```sh
cargo add serde --features derive
cargo add serde_yaml
```

### 定义提取器

`HttpRequest::body()` 可以取到请求体。普通文本类请求体会以 `RequestBody::Simple` 保存，因此我们可以取出字节并交给 `serde_yaml::from_slice`。

```rust
use faithea::{
    data::{Json, inbound::FromRequest},
    handler::types::HttpHandlerError,
    post,
    request::{HttpRequest, RequestBody, TryFromRequest},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

struct YamlBody<T>(T);

impl<T> YamlBody<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<'a, T> TryFromRequest<'a> for YamlBody<T>
where
    T: DeserializeOwned,
{
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        match req.body() {
            Some(RequestBody::Simple(body)) => {
                let value = serde_yaml::from_slice::<T>(body.as_ref())
                    .map_err(|_| {
                        HttpHandlerError::before_handler_incompatible_request_body_type()
                    })?;

                Ok(YamlBody(value))
            }
            _ => Err(HttpHandlerError::before_handler_empty_request_body()),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct DeployConfig {
    service: String,
    replicas: u8,
    public: bool,
}
```

这里的 `YamlBody<T>` 是一个通用提取器。只要 `T` 实现 `DeserializeOwned`，就可以从 YAML body 中解析出来。

### 使用提取器

在 handler 参数中使用 `FromRequest<YamlBody<DeployConfig>>`。解析成功后，把内部配置取出来返回为 JSON，方便测试结果。

```rust
#[post("/deploy-config")]
async fn deploy_config(config: FromRequest<YamlBody<DeployConfig>>) {
    Json(config.into_inner().into_inner())
}
```

### 测试提取器

先写一个 YAML 文件：

```sh
cat > deploy.yaml <<'YAML'
service: api
replicas: 3
public: true
YAML
```

再发送请求：

```sh
curl -i -X POST http://127.0.0.1:3000/deploy-config \
  -H "content-type: application/x-yaml" \
  --data-binary @deploy.yaml
```

如果 YAML 格式正确，handler 会返回解析后的 JSON。格式错误时，提取器会在 handler 运行前返回请求体不兼容错误。

## 合适的提取职责

自定义提取器适合封装当前登录用户、请求 ID、经过验证的 Header 和其他类型化请求上下文。

提取器应当保持确定性和低开销。耗时的业务操作应在提取成功后交给 service 层。
