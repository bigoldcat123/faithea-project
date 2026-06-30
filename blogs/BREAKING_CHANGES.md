# Breaking Changes

This document lists all breaking changes between the README documentation and the current API (v0.1.9).

---

## 1. `TryFromParam` trait signature changed

**Before:**
```rust
impl TryFromParam<'_> for Age {
    fn try_from_param(value: &String) -> Result<Self, HttpHandlerError> {
```

**After:**
```rust
impl TryFromParam<'_> for MyAge {
    fn try_from_param(value: &str) -> Result<Self, ParseHandlerParamError> {
```

The `value` parameter type changed from `&String` to `&str`. The error type also changed from `HttpHandlerError` to `ParseHandlerParamError`.

---

## 2. `#[search_param]` now supports renaming

**Before:**
```rust
#[get("/search")]
async fn search(
    #[search_param] name: String,
    #[search_param] age: Option<String>
) -> String {
```

**After:**
```rust
#[get("/searchParam")]
pub async fn search_param(
    #[search_param("Name")] name: &str,
    #[search_param] age: Option<String>
) {
```

The `#[search_param]` attribute now accepts an optional string literal to specify the query parameter name, allowing it to differ from the Rust variable name. Additionally, `&str` is now a supported parameter type.

---

## 3. `ConvertError` path changed

**Before:**
```rust
use faithea::{get, TryFromParam};
// ConvertError was in an implicit location
```

**After:**
```rust
use faithea::{
    get,
    request::{ConvertError, TryFromParam, error::ParseHandlerParamError},
};
```

`ConvertError` is now at `faithea::request::ConvertError`. `ParseHandlerParamError` is at `faithea::request::error::ParseHandlerParamError`.

---

## 4. `HttpHandlerError` type alias location

**Before:**
```rust
use HttpHandlerError; // ambiguous path
```

**After:**
```rust
use faithea::handler::types::HttpHandlerError;
// or equivalently:
use faithea::error::Error; // HttpHandlerError = Error
```

---

## 5. `HttpServer::builder()` static map — new method

**New:** The builder now has a `.static_map()` method for serving static files:

```rust
HttpServer::builder()
    .static_map(
        "/static/**",
        "/path/to/static/directory",
    )
```

Alternatively, the standalone `static_map` utility function still works:
```rust
use faithea::{get, util::static_map};

#[get("/**")]
pub async fn file_map() {
    static_map(_req, "/path/to/static/directory").await
}
```

---

## 6. Handler return type changed — `impl HttpResponseModifier`

**Before:** Handlers returned concrete types like `String`, `Json<T>`, `()`.

**After:** The macro adds `-> impl faithea::response::HttpResponseModifier` as the return type automatically. All return values must implement `HttpResponseModifier`. This is transparent when using macros — just return supported types directly.

```rust
#[get("/")]
async fn hello_world() {
    res_modifiers!("Hello,World", CORS)
}
```

---

## 7. New features: SSE, Stream, Redirect, FromRequest, Cookies

### SSE (Server-Sent Events)
```rust
use faithea::{get, res_modifiers, response::{sse::SSE, stream::Stream}};
use bytes::Bytes;

#[get("/stream")]
async fn stream() {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(10);
    // ... spawn task that sends to tx ...
    res_modifiers!(Stream::new(rx), SSE)
}
```

### Redirect
```rust
use faithea::{get, response::redirect::Redirect};

#[get("/redirect")]
async fn redirect() {
    Redirect("https://www.example.com")
}
```

### FromRequest (custom parameter extraction)
```rust
use faithea::{data::inbound::FromRequest, request::TryFromRequest};

impl<'a> TryFromRequest<'a> for MyType {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, ParseHandlerParamError> {
        // custom extraction logic
    }
}

#[post("/fromRequest")]
pub async fn from_request(stu: FromRequest<MyType>) {
    Json(stu.into_inner())
}
```

### Cookies
```rust
#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
```

---

## 8. Custom `HttpResponseModifier` implementation

You can now implement `HttpResponseModifier` for your own types:

```rust
use faithea::{
    header::{CONTENT_LENGTH, HeaderValue},
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture},
};

struct MyCustomType { name: String }

impl HttpResponseModifier for MyCustomType {
    fn modify<'a>(&'a mut self, res: &'a mut HttpResponse) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header("x-custom-header", self.name.parse().unwrap());
            Ok(())
        })
    }
}
```

---

## 9. `MultipartData` derive — rename attribute

**New:** The `MultipartData` derive macro supports `#[faithea(rename = "...")]`:

```rust
#[derive(MultipartData)]
struct StuInfo {
    #[faithea(rename = "otherInfo")]
    pub other_info: A,
    pub name: Vec<String>,
    pub age: i32,
}
```

---

## 10. `Json<T>` handler parameter — no destructuring needed

**Before:**
```rust
async fn create_user(Json(user): Json<User>) -> Json<User> {
    Json(user)
}
```

**After** (simpler, both forms work):
```rust
#[post("/json")]
async fn json_test(stu: Json<StuData>) {
    stu  // Json<T> implements HttpResponseModifier and Deref<Target = T>
}
```

`Json<T>` implements `Deref<Target = T>`, so you can use `stu.field` directly without destructuring.

---

## 11. Tokio dependency — add `macros` feature

**Before:**
```toml
tokio = { version = "1.48.0", features = ["rt-multi-thread"] }
```

**After:**
```toml
tokio = { version = "1.48.0", features = ["rt-multi-thread", "macros"] }
```

The `macros` feature is required for `#[tokio::main]`.

---

## 12. Version Update

The crate version has been updated from `0.1.6`/`0.1.8` to `0.1.9`.

---

## Migration Checklist

- [ ] Update `TryFromParam` implementation to use `&str` instead of `&String`
- [ ] Update `ConvertError` imports to `faithea::request::ConvertError`
- [ ] Rename query parameters using `#[search_param("name")]` as needed
- [ ] Use `&str` for search params where appropriate
- [ ] Add `tokio` `macros` feature to your `Cargo.toml`
- [ ] Update `faithea` version to `0.1.9`
- [ ] Consider using `.static_map()` builder method for static file serving
- [ ] Explore new features: SSE, Redirect, FromRequest, Cookies
