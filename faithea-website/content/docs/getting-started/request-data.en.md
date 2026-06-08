---
title: Request Data
description: Read parameters, JSON bodies, multipart forms, files, and request metadata.
---

Faithea extracts request data directly into handler arguments. The handler signature describes the data a route expects.

## Path parameters

Path parameters use the same name in the route and handler. Faithea converts the URL value into the declared Rust type before calling the handler.

### Define routes

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

### Mount routes

```rust
.mount(
    "/api",
    handlers!(get_user),
)
```

### Test with curl

```sh
curl http://127.0.0.1:3000/api/users/42
```

## Query parameters

Mark query parameters with `#[search_param]`. A required parameter produces an error when it is missing or invalid. Wrap a parameter in `Option<T>` when it is optional.

Use `#[search_param("Name")]` when the query-string key differs from the Rust argument name.

### Define routes

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

### Mount routes

```rust
.mount(
    "/api",
    handlers!(list_users, search),
)
```

### Test with curl

```sh
curl "http://127.0.0.1:3000/api/users?page=2&keyword=rust"
curl "http://127.0.0.1:3000/api/search?Name=Ada"
```

## JSON bodies

Wrap a type in `Json<T>` to parse a JSON request body. Faithea parses the body before the handler runs. Returning the same `Json<T>` value sends it back as a JSON response.

Add Serde to derive request and response serialization:

```sh
cargo add serde --features derive
```

### Define routes

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

### Mount routes

```rust
.mount(
    "/api",
    handlers!(create_user),
)
```

### Test with curl

```sh
curl -X POST http://127.0.0.1:3000/api/users \
  -H "content-type: application/json" \
  -d '{"name":"Ada","age":36}'
```

## Multipart forms and files

Faithea parses multipart forms into typed Rust structures using `Multipart<T>` and the `MultipartData` derive macro.

Use `Option<T>` for optional fields and `Vec<T>` for repeated fields or multiple files. Rename a form field with `#[faithea(rename = "...")]` when it differs from the Rust field name.

Uploaded files are stored in temporary paths. `MultiPartFile` removes its temporary file when the value is dropped, so move or copy files you need to retain.

### Define routes

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

### Mount routes

```rust
.mount(
    "/api",
    handlers!(upload),
)
```

### Test with curl

Create a 5M mock upload file first:

```sh
dd if=/dev/zero of=mock-upload.bin bs=1m count=5
```

Then send the multipart request:

```sh
curl -X POST http://127.0.0.1:3000/api/upload \
  -F "displayName=Ada" \
  -F "public=true" \
  -F "tags=rust" \
  -F "tags=web" \
  -F "files=@mock-upload.bin"
```

## Custom multipart fields

Implement `TryFromPart` when a multipart field needs custom conversion. Any type that implements `TryFromPart` can be placed inside a `MultipartData` struct.

### Define routes

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

### Mount routes

```rust
.mount(
    "/api",
    handlers!(create_label),
)
```

### Test with curl

```sh
curl -X POST http://127.0.0.1:3000/api/labels \
  -F "label=release"
```

## Request metadata

Every route handler has access to an injected `_req` value. Use it when you need the URI, headers, cookies, or lower-level request information.

The `_req` argument is supplied by the route macro, so it does not appear in the function signature you write.

### Define routes

```rust
use faithea::get;

#[get("/request-info")]
async fn request_info() {
    format!("uri: {}", _req.uri())
}
```

### Mount routes

```rust
.mount(
    "/api",
    handlers!(request_info),
)
```

### Test with curl

```sh
curl "http://127.0.0.1:3000/api/request-info?debug=true"
```

Continue with [Responses](./responses.md) to control response bodies, headers, and status codes.
