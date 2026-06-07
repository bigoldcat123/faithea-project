---
title: 请求数据
description: 读取参数、JSON 请求体、Multipart 表单、文件和请求元数据。
---

Faithea 会将请求数据直接提取到 handler 参数中。Handler 函数签名描述了路由期望接收的数据。

## 路径参数

路径参数在路由和 handler 中使用相同名称：

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

Faithea 会在调用 handler 前，将 URL 中的值转换成声明的 Rust 类型。

## 查询参数

使用 `#[search_param]` 标记查询参数：

```rust
#[get("/users")]
async fn list_users(
    #[search_param] page: u32,
    #[search_param] keyword: Option<String>,
) {
    format!("page={page}, keyword={keyword:?}")
}
```

使用以下请求访问路由：

```sh
curl "http://127.0.0.1:3000/users?page=2&keyword=rust"
```

必填参数缺失或无效时会产生错误。需要可选参数时，将类型包装为 `Option<T>`。

当查询字符串键名与 Rust 参数名不同时，可以使用 `#[search_param("Name")]`：

```rust
#[get("/search")]
async fn search(#[search_param("Name")] name: String) {
    name
}
```

## JSON 请求体

添加 Serde，以派生请求和响应的序列化实现：

```sh
cargo add serde --features derive
```

使用 `Json<T>` 包装类型，以解析 JSON 请求体：

```rust
use faithea::{data::Json, post};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct CreateUser {
    name: String,
    age: u8,
}

#[post("/users")]
async fn create_user(user: Json<CreateUser>) {
    user
}
```

发送 JSON 请求：

```sh
curl -X POST http://127.0.0.1:3000/users \
  -H "content-type: application/json" \
  -d '{"name":"Ada","age":36}'
```

Faithea 会在 handler 运行前解析请求体。返回同一个 `Json<T>` 值会将它作为 JSON 响应发送。

## Multipart 表单与文件

Faithea 使用 `Multipart<T>` 和 `MultipartData` 派生宏，将 Multipart 表单解析为类型化 Rust 结构：

```rust
use faithea::{
    MultipartData, post,
    data::inbound::multipart::{MultiPartFile, Multipart},
};

#[derive(MultipartData, Debug)]
struct UploadForm {
    title: String,
    public: Option<bool>,
    tags: Vec<String>,
    files: Vec<MultiPartFile>,
}

#[post("/upload")]
async fn upload(form: Multipart<UploadForm>) {
    format!(
        "title={}, tags={}, files={}",
        form.title,
        form.tags.len(),
        form.files.len(),
    )
}
```

使用 `Option<T>` 表示可选字段，使用 `Vec<T>` 表示重复字段或多个文件。

当表单字段与 Rust 字段名称不同时，可以重命名字段：

```rust
#[derive(MultipartData)]
struct ProfileForm {
    #[faithea(rename = "displayName")]
    display_name: String,
}
```

使用 `curl` 发送 Multipart 请求：

```sh
curl -X POST http://127.0.0.1:3000/upload \
  -F "title=release" \
  -F "public=true" \
  -F "tags=rust" \
  -F "tags=web" \
  -F "files=@README.md"
```

上传文件会存储在临时路径中。`MultiPartFile` 被释放时会删除临时文件，因此需要保留的文件必须及时移动或复制。

## 自定义 Multipart 字段

Multipart 字段需要自定义转换时，实现 `TryFromPart`：

```rust
use faithea::{
    data::inbound::multipart::{Part, TryFromPart},
    handler::types::HttpHandlerError,
};

struct Label(String);

impl TryFromPart for Label {
    fn try_from_part(part: Part) -> Result<Self, HttpHandlerError> {
        match part {
            Part::Lit(value) => Ok(Label(value)),
            Part::File(_) => Err(HttpHandlerError::before_handler_incompatible_request_body_type()),
        }
    }
}
```

## 请求元数据

每个路由 handler 都可以访问自动注入的 `_req` 值。需要 URI、Header、Cookie 或更底层的请求信息时，可以使用它：

```rust
#[get("/request-info")]
async fn request_info() {
    format!("uri: {}", _req.uri())
}
```

`_req` 参数由路由宏提供，因此不需要出现在你编写的函数签名中。

继续阅读[响应](./responses.md)，了解如何控制响应体、Header 和状态码。
