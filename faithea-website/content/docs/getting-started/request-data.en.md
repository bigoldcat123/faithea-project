---
title: Request Data
description: Read parameters, JSON bodies, multipart forms, files, and request metadata.
---

Faithea extracts request data directly into handler arguments. The handler signature describes the data a route expects.

## Path parameters

Path parameters use the same name in the route and handler:

```rust
use faithea::get;

#[get("/users/{id}")]
async fn get_user(id: u64) {
    format!("user {id}")
}
```

Faithea converts the URL value into the declared Rust type before calling the handler.

## Query parameters

Mark query parameters with `#[search_param]`:

```rust
#[get("/users")]
async fn list_users(
    #[search_param] page: u32,
    #[search_param] keyword: Option<String>,
) {
    format!("page={page}, keyword={keyword:?}")
}
```

Call the route with:

```sh
curl "http://127.0.0.1:3000/users?page=2&keyword=rust"
```

A required parameter produces an error when it is missing or invalid. Wrap a parameter in `Option<T>` when it is optional.

Use `#[search_param("Name")]` when the query-string key differs from the Rust argument name:

```rust
#[get("/search")]
async fn search(#[search_param("Name")] name: String) {
    name
}
```

## JSON bodies

Add Serde to derive request and response serialization:

```sh
cargo add serde --features derive
```

Wrap a type in `Json<T>` to parse a JSON request body:

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

Send a JSON request:

```sh
curl -X POST http://127.0.0.1:3000/users \
  -H "content-type: application/json" \
  -d '{"name":"Ada","age":36}'
```

Faithea parses the body before the handler runs. Returning the same `Json<T>` value sends it back as a JSON response.

## Multipart forms and files

Faithea parses multipart forms into typed Rust structures using `Multipart<T>` and the `MultipartData` derive macro:

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

Use `Option<T>` for optional fields and `Vec<T>` for repeated fields or multiple files.

Rename a form field when it differs from the Rust field name:

```rust
#[derive(MultipartData)]
struct ProfileForm {
    #[faithea(rename = "displayName")]
    display_name: String,
}
```

Send a multipart request with `curl`:

```sh
curl -X POST http://127.0.0.1:3000/upload \
  -F "title=release" \
  -F "public=true" \
  -F "tags=rust" \
  -F "tags=web" \
  -F "files=@README.md"
```

Uploaded files are stored in temporary paths. `MultiPartFile` removes its temporary file when the value is dropped, so move or copy files you need to retain.

## Custom multipart fields

Implement `TryFromPart` when a multipart field needs custom conversion:

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

## Request metadata

Every route handler has access to an injected `_req` value. Use it when you need the URI, headers, cookies, or lower-level request information:

```rust
#[get("/request-info")]
async fn request_info() {
    format!("uri: {}", _req.uri())
}
```

The `_req` argument is supplied by the route macro, so it does not appear in the function signature you write.

Continue with [Responses](./responses.md) to control response bodies, headers, and status codes.
