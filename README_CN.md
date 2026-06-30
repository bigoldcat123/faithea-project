# Faithea

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)](https://www.rust-lang.org)
[![Crates.io](https://img.shields.io/crates/v/faithea.svg)](https://crates.io/crates/faithea)

一个轻量、高性能的异步HTTP框架，用纯Rust构建。支持HTTP/1.1和HTTP/2，提供强大的路由系统、WebSocket支持和灵活的数据提取机制。

## ✨ 核心特性

- 🚀 **异步高性能** - 基于Tokio构建，提供出色的并发性能
- 🌐 **双协议支持** - 同时支持HTTP/1.1和HTTP/2
- 🔌 **WebSocket支持** - 内置WebSocket服务器功能
- 🛡️ **类型安全** - 利用Rust类型系统确保编译时安全
- 🎯 **灵活路由** - 支持精确匹配、参数化路径和通配符路由
- 📦 **智能数据提取** - JSON、Multipart、查询参数、路径参数自动解析
- 🔐 **Guard中间件** - 可链式组合的请求验证和认证中间件
- 🌍 **CORS支持** - 内置跨域资源共享支持
- 🔒 **TLS/HTTPS** - 完整的加密连接支持
- ⚠️ **全局错误处理** - 统一的错误处理机制
- 📡 **SSE流式传输** - 内置服务端推送事件（Server-Sent Events）支持
- 📁 **静态文件服务** - 通过通配符路径提供静态文件
- 🍪 **Cookie支持** - 解析和处理Cookie
- 🔀 **重定向支持** - 内置HTTP重定向响应

## 📦 项目结构

```
faithea-project/
├── faithea/              # 核心HTTP框架库
│   ├── src/
│   │   ├── server/      # HTTP/1.1和HTTP/2服务器实现
│   │   ├── handler/     # 请求处理器和路由系统
│   │   ├── websocket/   # WebSocket重导出
│   │   ├── data/        # 数据提取和转换（JSON, Multipart等）
│   │   ├── guard/       # 中间件Guard系统
│   │   ├── request/     # HTTP请求解析
│   │   ├── response/    # HTTP响应构建
│   │   └── route/       # 路由模式匹配
│   └── Cargo.toml
├── faithea-macro/       # 过程宏库（路由装饰器等）
│   └── Cargo.toml
├── faithea-websocket/   # WebSocket实现
│   └── Cargo.toml
├── faithea-io-core/     # 异步IO抽象
│   └── Cargo.toml
├── app/                 # 示例应用
│   ├── src/
│   │   ├── main.rs      # 示例服务器
│   │   ├── ws/          # WebSocket示例
│   │   ├── test_handler/# 所有handler示例
│   │   └── util/        # 工具函数
│   └── Cargo.toml
└── Cargo.toml           # Workspace配置
```

## 🚀 快速开始

### 安装依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
faithea = "0.1.9"
tokio = { version = "1.48.0", features = ["rt-multi-thread", "macros"] }
serde = { version = "1.0", features = ["derive"] }
```

### 第一个服务器

```rust
use faithea::{get, handlers, HttpServer, res_modifiers, response::cors::CORS};

#[get("/")]
async fn hello_world() {
    res_modifiers!("Hello, Faithea!", CORS)
}

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(hello_world))
        .host("127.0.0.1")
        .port(8080)
        .build()
        .run()
        .await
        .unwrap();
}
```

运行服务器：
```bash
cargo run
```

访问 `http://127.0.0.1:8080/` 即可看到响应！

## 📖 使用指南

### 路由定义

使用宏轻松定义路由：

```rust
use faithea::{get, post, put, delete, handlers, HttpServer};

#[get("/")]
async fn index() {
    "Welcome to Faithea!"
}

#[get("/users/{id}")]
async fn get_user(id: String) {
    format!("User ID: {}", id)
}

#[post("/users")]
async fn create_user() {
    "User created!"
}

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(index, get_user, create_user))
        .host("127.0.0.1")
        .port(8080)
        .build()
        .run()
        .await
        .unwrap();
}
```

### JSON 数据处理

```rust
use serde::{Deserialize, Serialize};
use faithea::{data::Json, post};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    age: i32,
}

#[post("/users")]
async fn create_user(user: Json<User>) {
    user
}
```

> **注意：** `Json<T>` 实现了 `Deref<Target = T>`，可直接使用 `user.name` 访问字段，无需解构。

### Multipart 文件上传

```rust
use faithea::{
    data::inbound::multipart::{MultiPartFile, Multipart, Part, TryFromParts},
    error::MultipartError, MultipartData, post,
};

#[derive(Debug)]
struct CustomPart {
    pub value: String,
}

impl TryFromParts for CustomPart {
    fn try_from_parts(part: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        if let Some(mut part) = part
            && let Some(Part::Lit(s)) = part.pop()
        {
            Ok(Self { value: s })
        } else {
            Err(MultipartError::FieldNotExist)
        }
    }
}

#[derive(MultipartData)]
struct UploadData {
    #[faithea(rename = "otherInfo")]
    pub other_info: CustomPart,
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: Vec<MultiPartFile>,
}

#[post("/upload")]
async fn upload(data: Multipart<UploadData>) {
    format!(
        "name: {:?}, age: {}, files: {}",
        data.name,
        data.age,
        data.profile.len()
    )
}
```

### 查询参数和路径参数

```rust
use faithea::{
    get,
    request::{ConvertError, TryFromParam, error::ParseHandlerParamError},
};

#[derive(Debug)]
struct MyAge {
    value: i32,
}

impl TryFromParam<'_> for MyAge {
    fn try_from_param(value: &str) -> Result<Self, ParseHandlerParamError> {
        let a = value.parse::<i32>().map_err(|_| ConvertError {
            from: value.into(),
            to: "MyAge".into(),
        })?;
        Ok(Self { value: a })
    }
}

#[get("/search")]
async fn search(
    #[search_param("Name")] name: &str,
    #[search_param] age: Option<String>,
) {
    format!("搜索：{}, 年龄：{:?}", name, age)
}

#[get("/users/{name}/{age}")]
async fn get_user(name: String, age: MyAge) {
    format!("用户 {}, 年龄 {:?}", name, age.value)
}
```

> **注意：** `#[search_param("Name")]` 允许查询参数键名与Rust变量名不同。单独使用 `#[search_param]` 则以变量名作为键名。`&str` 现在是支持的参数类型。

### WebSocket 支持

```rust
use std::{collections::HashMap, sync::LazyLock};
use faithea::{
    request::HttpRequest,
    websocket::{data::WebSocketDataPayLoad, socket::WebSocket},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc::Sender};

static WS_SENDERS: LazyLock<Mutex<HashMap<String, Sender<WebSocketDataPayLoad>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
struct WsDataMessage {
    r#type: String,
    to: String,
    from: String,
    content: String,
}

pub async fn ws(websocket: WebSocket, req: HttpRequest) {
    let (mut r, s) = websocket.split();
    let name = req.get_pathparam("name").unwrap();
    {
        let mut map = WS_SENDERS.lock().await;
        map.insert(name.clone(), s.clone());
    }
    while let Some(msg) = r.recv().await {
        if let Ok(data) = serde_json::from_slice::<WsDataMessage>(msg.as_bytes()) {
            let map = WS_SENDERS.lock().await;
            if let Some(sender) = map.get(&data.to) {
                let a: String = serde_json::to_string(&data).unwrap();
                sender.send(WebSocketDataPayLoad::text(a)).await.unwrap();
            }
        }
    }
}

// 在main中注册：
HttpServer::builder()
    .websocket("/ws/{name}", ws)
    // ...其他配置
```

### SSE 流式传输

```rust
use std::time::Duration;
use bytes::Bytes;
use faithea::{get, res_modifiers, response::{sse::SSE, stream::Stream}};
use tokio::time::sleep;

#[get("/stream")]
async fn stream() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(10);
    tokio::spawn(async move {
        for _ in 0..10 {
            tx.send(Bytes::from("data: hello\n")).await.unwrap();
            sleep(Duration::from_millis(100)).await;
        }
    });

    res_modifiers!(Stream::new(rx), SSE)
}
```

### 静态文件服务

使用 `.static_map()` 构建器方法：

```rust
HttpServer::builder()
    .static_map(
        "/static/**",
        "/path/to/static/directory",
    )
```

或者使用独立的工具函数：

```rust
use faithea::{get, util::static_map};

#[get("/**")]
pub async fn file_map() {
    static_map(_req, "/path/to/static/directory").await
}
```

### Cookie 处理

```rust
use faithea::get;

#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
```

`_req` 参数由路由宏自动注入，提供对完整 `HttpRequest` 的访问。

### 重定向

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/redirect")]
async fn redirect() {
    Redirect("https://www.example.com")
}
```

### 自定义数据提取（FromRequest）

为自定义类型实现 `TryFromRequest`：

```rust
use serde::{Deserialize, Serialize};
use faithea::{
    data::inbound::FromRequest,
    data::Json,
    post,
    request::{HttpRequest, TryFromRequest, error::ParseHandlerParamError},
};

#[derive(Debug, Serialize, Deserialize)]
struct MyData {
    name: String,
    age: i32,
}

impl<'a> TryFromRequest<'a> for MyData {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, ParseHandlerParamError> {
        // 自定义提取逻辑 — 读取请求头、请求体等
        Ok(MyData {
            name: "来自请求".into(),
            age: 42,
        })
    }
}

#[post("/fromRequest")]
async fn from_request(data: FromRequest<MyData>) {
    Json(data.into_inner())
}
```

### 中间件 Guards

```rust
use faithea::HttpServer;

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .guard("/api/**", async |req| {
            // 验证API密钥
            if req.get_header("Authorization").is_some() {
                Ok(req)
            } else {
                Err(HttpResponse::unauthorized())
            }
        })
        .guard("/**", async |req| Ok(req))  // 放行所有其他请求
        .mount("/api", handlers!(api_handler))
        // ...其他配置
}
```

### 全局错误处理

```rust
use faithea::{HttpServer, res_modifiers};

HttpServer::builder()
    .globale_error_handler(async |e: faithea::error::Error| {
        res_modifiers!(format!("Error: {:?}", e))
    })
    // ...其他配置
```

### CORS 和 HTTPS

```rust
HttpServer::builder()
    .cors()  // 启用CORS
    .tls("key.pem", "cert.pem")  // 启用TLS
    .h2()  // 启用HTTP/2
    .port(443)
    // ...其他配置
```

### 自定义响应修改器

你可以为自己的类型实现 `HttpResponseModifier`：

```rust
use faithea::{
    get, res_modifiers,
    header::{CONTENT_LENGTH, HeaderValue},
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture, cors::CORS},
};

struct CustomHeader {
    name: String,
}

impl HttpResponseModifier for CustomHeader {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header("x-custom-header", self.name.parse().unwrap());
            Ok(())
        })
    }
}

#[get("/custom")]
async fn custom_res() {
    res_modifiers!(
        CustomHeader { name: "Hello".into() },
        CORS,
    )
}
```

## 🔧 技术栈

- **[Tokio](https://tokio.rs/)** - 异步运行时
- **[Serde](https://serde.rs/)** - 序列化/反序列化
- **[http](https://github.com/hyperium/http)** - HTTP类型定义
- **[rustls](https://github.com/rustls/rustls)** - TLS实现
- **[hyper](https://github.com/hyperium/hyper)** - HTTP/1.1和HTTP/2服务

## 📝 示例项目

查看 `app/` 目录下的完整示例：
- **基础路由**：多种路由类型的演示
- **数据处理**：JSON、Multipart等数据格式处理
- **WebSocket**：实时通信示例
- **中间件**：认证和Guard示例
- **SSE和流式传输**：服务端推送事件和流式响应
- **静态文件**：通配符路由的静态文件服务
- **自定义提取器**：FromRequest和自定义响应修改器

## 🤝 贡献

欢迎贡献！请随时提交 Issue 或 Pull Request。

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 双重许可证。您可以自由选择使用其中一种。

---

**Made with ❤️ by the Faithea Team**
