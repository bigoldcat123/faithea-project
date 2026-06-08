---
title: 请求数据
description: 读取参数、JSON 请求体、Multipart 表单、文件和请求元数据。
---

Faithea 会将请求数据直接提取到 handler 参数中。Handler 函数签名描述了路由期望接收的数据。

## 路径参数

路径参数在路由和 handler 中使用相同名称。Faithea 会在调用 handler 前，将 URL 中的值转换成声明的 Rust 类型。

### 定义路由

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(get_user),
)
```

### 测试 curl

```sh
curl http://127.0.0.1:3000/api/users/42
```

## 查询参数

使用 `#[search_param]` 标记查询参数。必填参数缺失或无效时会产生错误。需要可选参数时，将类型包装为 `Option<T>`。

当查询字符串键名与 Rust 参数名不同时，可以使用 `#[search_param("Name")]`。

### 定义路由

```rust
use faithea::get;

#[get("/users")]
async fn list_users(
    #[search_param] page: u32,
    #[search_param] keyword: Option<String>,
) {
    format!("page={page}, keyword={keyword:?}")
}

#[get("/search")]
async fn search(#[search_param("Name")] name: String) {
    name
}
```

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(list_users, search),
)
```

### 测试 curl

```sh
curl "http://127.0.0.1:3000/api/users?page=2&keyword=rust"
curl "http://127.0.0.1:3000/api/search?Name=Ada"
```

## JSON 请求体

使用 `Json<T>` 包装类型，以解析 JSON 请求体。Faithea 会在 handler 运行前解析请求体。返回同一个 `Json<T>` 值会将它作为 JSON 响应发送。

添加 Serde，以派生请求和响应的序列化实现：

```sh
cargo add serde --features derive
```

### 定义路由

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

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(create_user),
)
```

### 测试 curl

```sh
curl -X POST http://127.0.0.1:3000/api/users \
  -H "content-type: application/json" \
  -d '{"name":"Ada","age":36}'
```

## Multipart 表单与文件

Faithea 使用 `Multipart<T>` 和 `MultipartData` 派生宏，将 Multipart 表单解析为类型化 Rust 结构。

使用 `Option<T>` 表示可选字段，使用 `Vec<T>` 表示重复字段或多个文件。表单字段与 Rust 字段名称不同时，可以使用 `#[faithea(rename = "...")]` 重命名。

上传文件会存储在临时路径中。`MultiPartFile` 被释放时会删除临时文件，因此需要保留的文件必须及时移动或复制。

### 定义路由

```rust
use faithea::{
    MultipartData, post,
    data::inbound::multipart::{MultiPartFile, Multipart},
};

#[derive(MultipartData, Debug)]
struct UploadForm {
    #[faithea(rename = "displayName")]
    display_name: String,
    public: Option<bool>,
    tags: Vec<String>,
    files: Vec<MultiPartFile>,
}

#[post("/upload")]
async fn upload(form: Multipart<UploadForm>) {
    format!(
        "displayName={}, tags={}, files={}",
        form.display_name,
        form.tags.len(),
        form.files.len(),
    )
}
```

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(upload),
)
```

### 测试 curl

先创建一个 5M 的 mock 上传文件：

```sh
dd if=/dev/zero of=mock-upload.bin bs=1m count=5
```

再发送 Multipart 请求：

```sh
curl -X POST http://127.0.0.1:3000/api/upload \
  -F "displayName=Ada" \
  -F "public=true" \
  -F "tags=rust" \
  -F "tags=web" \
  -F "files=@mock-upload.bin"
```

## 自定义 Multipart 字段

Multipart 字段需要自定义转换时，实现 `TryFromPart`。只要类型实现了 `TryFromPart`，就可以放入 `MultipartData` 结构中。

### 定义路由

```rust
use faithea::{
    MultipartData, post,
    data::inbound::multipart::{Multipart, Part, TryFromPart},
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

#[derive(MultipartData)]
struct LabelForm {
    label: Label,
}

#[post("/labels")]
async fn create_label(form: Multipart<LabelForm>) {
    format!("label={}", form.label.0)
}
```

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(create_label),
)
```

### 测试 curl

```sh
curl -X POST http://127.0.0.1:3000/api/labels \
  -F "label=release"
```

## 请求元数据

每个路由 handler 都可以访问自动注入的 `_req` 值。需要 URI、Header、Cookie 或更底层的请求信息时，可以使用它。

`_req` 参数由路由宏提供，因此不需要出现在你编写的函数签名中。

### 定义路由

```rust
use faithea::get;

#[get("/request-info")]
async fn request_info() {
    format!("uri: {}", _req.uri())
}
```

### 挂载路由

```rust
.mount(
    "/api",
    handlers!(request_info),
)
```

### 测试 curl

```sh
curl "http://127.0.0.1:3000/api/request-info?debug=true"
```

继续阅读[响应](./responses.md)，了解如何控制响应体、Header 和状态码。
