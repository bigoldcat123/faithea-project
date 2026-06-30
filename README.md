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
- 📡 **SSE Streaming** - Built-in Server-Sent Events support
- 📁 **Static File Serving** - Serve static files with wildcard paths
- 🍪 **Cookie Support** - Parse and handle cookies
- 🔀 **Redirect Support** - Built-in HTTP redirect responses

## 📦 Project Structure

```
faithea-project/
├── faithea/              # Core HTTP framework library
│   ├── src/
│   │   ├── server/      # HTTP/1.1 and HTTP/2 server implementation
│   │   ├── handler/     # Request handler and routing system
│   │   ├── websocket/   # WebSocket re-exports
│   │   ├── data/        # Data extraction and conversion (JSON, Multipart, etc.)
│   │   ├── guard/       # Middleware Guard system
│   │   ├── request/     # HTTP request parsing
│   │   ├── response/    # HTTP response building
│   │   └── route/       # Route pattern matching
│   └── Cargo.toml
├── faithea-macro/       # Procedural macro library (route decorators, etc.)
│   └── Cargo.toml
├── faithea-websocket/   # WebSocket implementation
│   └── Cargo.toml
├── faithea-io-core/     # Async I/O abstractions
│   └── Cargo.toml
├── app/                 # Example application
│   ├── src/
│   │   ├── main.rs      # Example server
│   │   ├── ws/          # WebSocket examples
│   │   ├── test_handler/# All handler examples
│   │   └── util/        # Utility functions
│   └── Cargo.toml
└── Cargo.toml           # Workspace configuration
```

## 🚀 Quick Start

### Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
faithea = "0.1.9"
tokio = { version = "1.48.0", features = ["rt-multi-thread", "macros"] }
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

### JSON Data Handling

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

> **Note:** `Json<T>` implements `Deref<Target = T>`, so you can access fields with `user.name` directly without destructuring.

### Multipart File Upload

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

### Query Parameters and Path Parameters

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
    format!("Searching for {}, age: {:?}", name, age)
}

#[get("/users/{name}/{age}")]
async fn get_user(name: String, age: MyAge) {
    format!("User {}, age {:?}", name, age.value)
}
```

> **Note:** `#[search_param("Name")]` allows the query parameter key to differ from the Rust variable name. Use `#[search_param]` alone to use the variable name as the key. `&str` is now a supported parameter type for both path and search params.

### WebSocket Support

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

// Register in main:
HttpServer::builder()
    .websocket("/ws/{name}", ws)
    // ... other configurations
```

### SSE Streaming (Server-Sent Events)

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

### Static File Serving

Use the `.static_map()` builder method:

```rust
HttpServer::builder()
    .static_map(
        "/static/**",
        "/path/to/static/directory",
    )
```

Alternatively, use the standalone utility function:

```rust
use faithea::{get, util::static_map};

#[get("/**")]
pub async fn file_map() {
    static_map(_req, "/path/to/static/directory").await
}
```

### Cookie Handling

```rust
use faithea::get;

#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
```

The `_req` parameter is automatically injected by the route macro and provides access to the full `HttpRequest`.

### Redirect

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/redirect")]
async fn redirect() {
    Redirect("https://www.example.com")
}
```

### Custom Data Extraction (FromRequest)

Implement `TryFromRequest` for custom types:

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
        // Custom extraction logic — read headers, body, etc.
        Ok(MyData {
            name: "from request".into(),
            age: 42,
        })
    }
}

#[post("/fromRequest")]
async fn from_request(data: FromRequest<MyData>) {
    Json(data.into_inner())
}
```

### Middleware Guards

```rust
use faithea::HttpServer;

#[tokio::main]
async fn main() {
    HttpServer::builder()
        .guard("/api/**", async |req| {
            // Validate API key
            if req.get_header("Authorization").is_some() {
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
use faithea::{HttpServer, res_modifiers};

HttpServer::builder()
    .globale_error_handler(async |e: faithea::error::Error| {
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

### Custom Response Modifiers

You can implement `HttpResponseModifier` for your own types:

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

## 🔧 Tech Stack

- **[Tokio](https://tokio.rs/)** - Async runtime
- **[Serde](https://serde.rs/)** - Serialization/deserialization
- **[http](https://github.com/hyperium/http)** - HTTP type definitions
- **[rustls](https://github.com/rustls/rustls)** - TLS implementation
- **[hyper](https://github.com/hyperium/hyper)** - HTTP/1.1 and HTTP/2 serving

## 📝 Examples

Check the `app/` directory for complete examples:
- **Basic Routing**: Various route type demonstrations
- **Data Processing**: JSON, Multipart, and other data format handling
- **WebSocket**: Real-time communication examples
- **Middleware**: Authentication and Guard examples
- **SSE & Streaming**: Server-Sent Events and streaming responses
- **Static Files**: Static file serving with wildcard routes
- **Custom Extractors**: FromRequest and custom response modifiers

## 🤝 Contributing

Contributions are welcome! Please feel free to submit Issues or Pull Requests.

## 📄 License

This project is dual-licensed under MIT OR Apache-2.0. You can choose whichever license you prefer.

---

**Made with ❤️ by the Faithea Team**
