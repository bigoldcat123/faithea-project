---
title: Custom Request Extractors
description: Convert an HttpRequest into reusable typed handler arguments.
---

Custom extractors move repeated request parsing out of handlers. Implement `TryFromRequest`, then receive the result through `FromRequest<T>`.

Faithea calls `TryFromRequest` before the handler. When extraction fails, the error follows the normal global error-handling flow.

## Token parsing

This example reads the Authorization header and wraps it as `CurrentUser`.

### Define the extractor

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

### Use the extractor

Wrap the custom type in `FromRequest<T>`:

```rust
#[get("/me")]
async fn me(user: FromRequest<CurrentUser>) {
    Json(user.into_inner())
}
```

### Test the extractor

First send a request without the Authorization header. It fails before reaching the handler:

```sh
curl -i http://127.0.0.1:3000/me
```

Then include the Authorization header. The request is extracted into `CurrentUser` and returned as JSON:

```sh
curl -i http://127.0.0.1:3000/me \
  -H "authorization: Bearer secret"
```

## YAML body parsing

Custom extractors can also parse request bodies. In this example, we will create `YamlBody<T>` so a handler can receive a YAML request body in the same spirit as `Json<T>`.

Add the dependencies first:

```sh
cargo add serde --features derive
cargo add serde_yaml
```

### Define the extractor

`HttpRequest::body()` gives access to the request body. Plain request bodies are stored as `RequestBody::Simple`, so the extractor can read those bytes and pass them to `serde_yaml::from_slice`.

```rust
use faithea::{
    data::{Json, inbound::FromRequest},
    handler::types::HttpHandlerError,
    post,
    request::{HttpRequest, RequestBody, TryFromRequest},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

struct YamlBody<T>(T);

impl<T> YamlBody<T> {
    fn into_inner(self) -> T {
        self.0
    }
}

impl<'a, T> TryFromRequest<'a> for YamlBody<T>
where
    T: DeserializeOwned,
{
    fn try_from_request(req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        match req.body() {
            Some(RequestBody::Simple(body)) => {
                let value = serde_yaml::from_slice::<T>(body.as_ref())
                    .map_err(|_| {
                        HttpHandlerError::before_handler_incompatible_request_body_type()
                    })?;

                Ok(YamlBody(value))
            }
            _ => Err(HttpHandlerError::before_handler_empty_request_body()),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct DeployConfig {
    service: String,
    replicas: u8,
    public: bool,
}
```

`YamlBody<T>` is generic. Any type that implements `DeserializeOwned` can be parsed from the YAML body.

### Use the extractor

Use `FromRequest<YamlBody<DeployConfig>>` in the handler arguments. After parsing, return the inner config as JSON so the result is easy to inspect.

```rust
#[post("/deploy-config")]
async fn deploy_config(config: FromRequest<YamlBody<DeployConfig>>) {
    Json(config.into_inner().into_inner())
}
```

### Test the extractor

Create a YAML file first:

```sh
cat > deploy.yaml <<'YAML'
service: api
replicas: 3
public: true
YAML
```

Then send the request:

```sh
curl -i -X POST http://127.0.0.1:3000/deploy-config \
  -H "content-type: application/x-yaml" \
  --data-binary @deploy.yaml
```

When the YAML is valid, the handler returns the parsed config as JSON. If the YAML is invalid, the extractor returns a request-body compatibility error before the handler runs.

## Good extractor responsibilities

Custom extractors work well for authenticated users, request IDs, validated headers, and other typed request context.

Keep extractors deterministic and inexpensive. Long-running business operations belong in the service layer after extraction succeeds.
