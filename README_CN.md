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

## 📦 项目结构

```
graduation/
├── faithea/              # 核心HTTP框架库
│   ├── src/
│   │   ├── server/      # HTTP/1.1和HTTP/2服务器实现
│   │   ├── handler/     # 请求处理器和路由系统
│   │   ├── websocket/   # WebSocket实现
│   │   ├── data/        # 数据提取和转换（JSON, Multipart等）
│   │   ├── guard/       # 中间件Guard系统
│   │   ├── request/     # HTTP请求解析
│   │   ├── response/    # HTTP响应构建
│   │   └── route/       # 路由模式匹配
│   └── Cargo.toml
├── faithea-macro/       # 过程宏库（路由装饰器等）
│   └── Cargo.toml
├── app/                 # 示例应用
│   ├── src/
│   │   ├── main.rs      # 示例服务器
│   │   ├── ws/          # WebSocket示例
│   │   └── util/        # 工具函数
│   └── Cargo.toml
└── Cargo.toml           # Workspace配置
```

## 🚀 快速开始

### 安装依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
faithea = "0.1.6"
tokio = { version = "1.48.0", features = ["rt-multi-thread"] }
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
use faithea::{get, post, put, delete, handlers};

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
use faithea::{post, data::Json};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    age: i32,
}

#[post("/users")]
async fn create_user(Json(user): Json<User>) -> Json<User> {
    Json(user)
}
```

### Multipart 文件上传

```rust
use faithea::{
    data::inbound::multipart::{MultiPartFile, Multipart},
    MultipartData, post,
};

#[derive(MultipartData)]
struct UploadData {
    pub username: String,
    pub profile: Vec<MultiPartFile>,
}

#[post("/upload")]
async fn upload(Multipart(data): Multipart<UploadData>) -> String {
    format!(
        "Uploaded {} files for user {}",
        data.profile.len(),
        data.username
    )
}
```

### 查询参数和路径参数

```rust
use faithea::{get, TryFromParam};

#[derive(Debug)]
struct Age {
    value: i32,
}

impl TryFromParam<'_> for Age {
    fn try_from_param(value: &String) -> Result<Self, HttpHandlerError> {
        Ok(Age {
            value: value.parse()?,
        })
    }
}

#[get("/search")]
async fn search(
    #[search_param] name: String,
    #[search_param] age: Option<String>
) -> String {
    format!("Searching for {}, age: {:?}", name, age)
}

#[get("/users/{id}")]
async fn get_user(id: String, age: Age) -> String {
    format!("User {}, age {}", id, age.value)
}
```

### WebSocket 支持

```rust

static WS_SENDERS: LazyLock<Mutex<HashMap<String, Sender<WebSocketDataPayLoad>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
struct WsDataMessage {
    r#type: String,
    to: String,
    from: String,
    content: String,
}

pub async fn ws(
    websocket: WebSocket,
    req: HttpRequest,
) {
    let  (mut r,s) = websocket.split();
    let name = req.get_pathparam("name").unwrap();
    {
        let mut map = WS_SENDERS.lock().await;
        map.insert(name.clone(), s.clone());
    }
    while let Some(msg) = r.recv().await {
        let data = serde_json::from_slice::<WsDataMessage>(msg.as_bytes()).unwrap();
        let map = WS_SENDERS.lock().await;
        if let Some(sender) = map.get(&data.to) {
            let a:String = serde_json::to_string(&data).unwrap();
            sender.send(WebSocketDataPayLoad::text(a)).await.unwrap();
        }
    }
}
// 在main中注册：
HttpServer::builder()
    .websocket("/ws/{name}", ws_handler)
    // ...其他配置
```

### 中间件 Guards

```rust
use faithea::HttpServer;

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .guard("/api/**", async |req| {
            // 验证API密钥
            if req.headers().get("Authorization").is_some() {
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
use faithea::HttpServer;

HttpServer::builder()
    .globale_error_handler(async |e: Error| {
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

## 🔧 技术栈

- **[Tokio](https://tokio.rs/)** - 异步运行时
- **[Serde](https://serde.rs/)** - 序列化/反序列化
- **[http](https://github.com/hyperium/http)** - HTTP类型定义
- **[rustls](https://github.com/rustls/rustls)** - TLS实现
- **[h2](https://github.com/hyperium/h2)** - HTTP/2实现

## 📝 示例项目

查看 `app/` 目录下的完整示例：
- **基础路由**：多种路由类型的演示
- **数据处理**：JSON、Multipart等数据格式处理
- **WebSocket**：实时通信示例
- **中间件**：认证和Guard示例

## 🤝 贡献

欢迎贡献！请随时提交 Issue 或 Pull Request。

## 📄 许可证

本项目采用 MIT 或 Apache-2.0 双重许可证。您可以自由选择使用其中一种。

---

**Made with ❤️ by the Faithea Team**
