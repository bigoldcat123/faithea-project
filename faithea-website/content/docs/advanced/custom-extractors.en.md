---
title: Custom Request Extractors
description: Convert an HttpRequest into reusable typed handler arguments.
---

Custom extractors move repeated request parsing out of handlers. Implement `TryFromRequest`, then receive the result through `FromRequest<T>`.

## Define an extractor

This extractor reads an authorization header:

```rust
use faithea::{
    data::{Json, inbound::FromRequest},
    get,
    handler::types::HttpHandlerError,
    request::{HttpRequest, TryFromRequest},
};
use serde::Serialize;

#[derive(Serialize)]
struct CurrentUser {
    token: String,
}

impl<'a> TryFromRequest<'a> for CurrentUser {
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        let token = req
            .get_header("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(HttpHandlerError::before_handler_param_not_exist)?;

        Ok(CurrentUser {
            token: token.to_string(),
        })
    }
}
```

## Use the extractor

Wrap the custom type in `FromRequest<T>`:

```rust
#[get("/me")]
async fn me(user: FromRequest<CurrentUser>) {
    Json(user.into_inner())
}
```

Faithea calls `TryFromRequest` before the handler. When extraction fails, the error follows the normal global error-handling flow.

## Good extractor responsibilities

Custom extractors work well for authenticated users, request IDs, validated headers, and other typed request context.

Keep extractors deterministic and inexpensive. Long-running business operations belong in the service layer after extraction succeeds.
