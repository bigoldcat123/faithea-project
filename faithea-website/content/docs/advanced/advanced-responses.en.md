---
title: Advanced Responses
description: Build redirects, cookies, and reusable custom response modifiers.
---

Every Faithea response is assembled from types that implement `HttpResponseModifier`. You can combine built-in modifiers or define your own.

## Redirect responses

Return `Redirect` to send a permanent redirect:

```rust
use faithea::{get, response::redirect::Redirect};

#[get("/old-page")]
async fn old_page() {
    Redirect("/new-page")
}
```

## Set a cookie

Build a response cookie and combine it with a body:

```rust
use faithea::{
    get, res_modifiers,
    response::cookie::{Cookie, CookieType},
};

#[get("/login")]
async fn login() {
    let mut cookie = Cookie::default();
    cookie.push(CookieType::KeyValue("session".into(), "abc123".into()));
    cookie.push(CookieType::Attribute("HttpOnly".into()));

    res_modifiers!(cookie, "logged in")
}
```

Use `_req.cookies()` inside a handler to inspect request cookies.

## Create a response modifier

Implement `HttpResponseModifier` for reusable response behavior:

```rust
use faithea::{
    get,
    header::HeaderValue,
    response::{HttpResponse, HttpResponseModifier, HttpResponseModifierFuture},
};

struct RequestId(&'static str);

impl HttpResponseModifier for RequestId {
    fn modify<'a>(&'a mut self, res: &'a mut HttpResponse) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header("x-request-id", HeaderValue::from_static(self.0));
            Ok(())
        })
    }
}

#[get("/custom")]
async fn custom() {
    faithea::res_modifiers!(RequestId("req-42"), "custom response")
}
```

Modifiers are applied in order. A custom modifier should update only the response properties it owns.
