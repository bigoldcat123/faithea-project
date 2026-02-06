# Faithea

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)](https://www.rust-lang.org)
[![Crates.io](https://img.shields.io/crates/v/faithea.svg)](https://crates.io/crates/faithea)

A lightweight, high-performance asynchronous HTTP framework built with pure Rust. Supports both HTTP/1.1 and HTTP/2, providing powerful routing systems, WebSocket support, and flexible data extraction mechanisms.

## ✨ Features

- 🚀 **Async High Performance** - Built on Tokio for excellent concurrency
- 🌐 **Dual Protocol Support** - HTTP/1.1 and HTTP/2 support
- 🔌 **WebSocket Support** - Built-in WebSocket server functionality
- 🛡️ **Type Safety** - Leverages Rust's type system for compile-time safety
- 🎯 **Flexible Routing** - Exact match, parameterized paths, and wildcard routes
- 📦 **Smart Data Extraction** - Automatic parsing for JSON, Multipart, query parameters, and path parameters
- 🔐 **Guard Middleware** - Chainable request validation and authentication middleware
- 🌍 **CORS Support** - Built-in Cross-Origin Resource Sharing support
- 🔒 **TLS/HTTPS** - Complete encrypted connection support
- ⚠️ **Global Error Handling** - Unified error handling mechanism

## 📦 Project Structure

```
graduation/
├── faithea/              # Core HTTP framework library
│   ├── src/
│   │   ├── server/      # HTTP/1.1 and HTTP/2 server implementation
│   │   ├── handler/     # Request handler and routing system
│   │   ├── websocket/   # WebSocket implementation
│   │   ├── data/        # Data extraction and conversion (JSON, Multipart, etc.)
│   │   ├── guard/       # Middleware Guard system
│   │   ├── request/     # HTTP request parsing
│   │   ├── response/    # HTTP response building
│   │   └── route/       # Route pattern matching
│   └── Cargo.toml
├── faithea-macro/       # Procedural macro library (route decorators, etc.)
│   └── Cargo.toml
├── app/                 # Example application
│   ├── src/
│   │   ├── main.rs      # Example server
│   │   ├── ws/          # WebSocket examples
│   │   └── util/        # Utility functions
│   └── Cargo.toml
└── Cargo.toml           # Workspace configuration
```

## 🚀 Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
faithea = "0.1.8"
tokio = { version = "1.48.0", features = ["rt-multi-thread","macros"] }
serde = { version = "1.0", features = ["derive"] }
```

### Your First Server

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

Run the server:
```bash
cargo run
```

Visit `http://127.0.0.1:8080/` to see the response!

## 📖 Usage Guide

### Route Definition

Define routes using macros:

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

### JSON Data Handling

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

### Multipart File Upload

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

### Query Parameters and Path Parameters

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

### WebSocket Support

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

// Register in main:
HttpServer::builder()
    .websocket("/ws/{name}", ws_handler)
    // ... other configurations
```

### Middleware Guards

```rust
use faithea::HttpServer;

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .guard("/api/**", async |req| {
            // Validate API key
            if req.headers().get("Authorization").is_some() {
                Ok(req)
            } else {
                Err(HttpResponse::unauthorized())
            }
        })
        .guard("/**", async |req| Ok(req))  // Allow all other requests
        .mount("/api", handlers!(api_handler))
        // ... other configurations
}
```

### Global Error Handler

```rust
use faithea::HttpServer;

HttpServer::builder()
    .globale_error_handler(async |e: Error| {
        res_modifiers!(format!("Error: {:?}", e))
    })
    // ... other configurations
```

### CORS and HTTPS

```rust
HttpServer::builder()
    .cors()  // Enable CORS
    .tls("key.pem", "cert.pem")  // Enable TLS
    .h2()  // Enable HTTP/2
    .port(443)
    // ... other configurations
```

## 🔧 Tech Stack

- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Serde](https://serde.rs/)** - Serialization/deserialization
- **[http](https://github.com/hyperium/http)** - HTTP type definitions
- **[rustls](https://github.com/rustls/rustls)** - TLS implementation
- **[h2](https://github.com/hyperium/h2)** - HTTP/2 implementation

## 📝 Examples

Check the `app/` directory for complete examples:
- **Basic Routing**: Various route type demonstrations
- **Data Processing**: JSON, Multipart, and other data format handling
- **WebSocket**: Real-time communication examples
- **Middleware**: Authentication and Guard examples

## 🤝 Contributing

Contributions are welcome! Please feel free to submit Issues or Pull Requests.

## 📄 License

This project is dual-licensed under MIT OR Apache-2.0. You can choose whichever license you prefer.

---

**Made with ❤️ by the Faithea Team**
