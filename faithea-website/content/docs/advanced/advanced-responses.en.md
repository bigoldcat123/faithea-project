---
title: Advanced Responses
description: Build redirects, cookies, and reusable custom response modifiers.
---

Every Faithea response is assembled from types that implement `HttpResponseModifier`. You can combine built-in modifiers or define your own.

## Redirect responses

Return `Redirect` to send a permanent redirect. This example redirects `/old-page` to `/new-page`.

### Define routes

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/old-page")]
async fn old_page() {
    Redirect("/new-page")
}

#[get("/new-page")]
async fn new_page() {
    "new page"
}
```

### Test with curl

Use `-i` to inspect the redirect status code and `location` header:

```sh
curl -i http://127.0.0.1:3000/old-page
```

Use `-L` to let curl follow the redirect:

```sh
curl -i -L http://127.0.0.1:3000/old-page
```

## Set a cookie

Cookies are response headers. Build a `HeaderMap`, set `SET_COOKIE`, then combine those headers with a body.

You can inspect request cookies inside a handler with `_req.cookies()`.

### Define routes

```rust
use faithea::{
    HeaderMap,
    get,
    header::{HeaderValue, SET_COOKIE},
    res_modifiers,
};

#[get("/login")]
async fn login() {
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        HeaderValue::from_static("session=abc123; HttpOnly; Path=/"),
    );

    res_modifiers!(headers, "logged in")
}

#[get("/profile")]
async fn profile() {
    format!("{:?}", _req.cookies())
}
```

### Test with curl

First call the login route and save the response cookie to `cookies.txt`:

```sh
curl -i -c cookies.txt http://127.0.0.1:3000/login
```

Then send the saved cookie to an endpoint that reads session data:

```sh
curl -i -b cookies.txt http://127.0.0.1:3000/profile
```

## Create a response modifier

In the previous guide, `YamlBody<T>` parsed YAML request bodies. Now we can implement `HttpResponseModifier` for the same wrapper so handlers can return YAML responses directly.

Add the dependencies first:

```sh
cargo add bytes
cargo add serde --features derive
cargo add serde_yaml
```

### Define the response modifier

`HttpResponseModifier` can update response headers, status, and body. Here we serialize the inner value to YAML, set `content-type` and `content-length`, then write the body.

```rust
use bytes::Bytes;
use faithea::{
    get,
    handler::types::HttpHandlerError,
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture, ResponseBody},
};
use serde::Serialize;

struct YamlBody<T>(T);

impl<T: Serialize + Send + Sync> HttpResponseModifier for YamlBody<T> {
    fn modify<'a>(&'a mut self, res: &'a mut HttpResponse) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            let body = serde_yaml::to_string(&self.0)
                .map_err(|_| HttpHandlerError::after_handler_incompatible_body_type())?;

            res.add_header("content-type", "application/x-yaml".parse()?);
            res.add_header("content-length", body.len().to_string().parse()?);
            res.set_body(ResponseBody::Simple(Bytes::from(body)));

            Ok(())
        })
    }
}
```

Modifiers are applied in order. A custom modifier should update only the response properties it owns.

### Use the response modifier

```rust
#[derive(Serialize)]
struct DeployConfig {
    service: String,
    replicas: u8,
    public: bool,
}

#[get("/deploy-config.yaml")]
async fn deploy_config_yaml() {
    YamlBody(DeployConfig {
        service: "api".into(),
        replicas: 3,
        public: true,
    })
}
```

### Test with curl

```sh
curl -i http://127.0.0.1:3000/deploy-config.yaml
```

The response body is YAML, and the response headers include `content-type: application/x-yaml`.
