---
title: TLS & HTTP/2
description: Serve HTTPS and enable HTTP/2 with a certificate and private key.
---

Faithea can terminate TLS directly and negotiate HTTP/2 using ALPN.

## Configure TLS

Provide a PEM private key and certificate:

```rust
use faithea::server::HttpServer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .tls("./certs/key.pem", "./certs/cert.pem")
        .build()
        .run()
        .await
        .unwrap();
}
```

Calling `.tls()` changes the default listener to `0.0.0.0:443`. Use `.host()` or `.port()` afterward when another address is required.

## Enable HTTP/2

Add `.h2()` to advertise HTTP/2 and HTTP/1.1:

```rust
HttpServer::builder()
    .tls("./certs/key.pem", "./certs/cert.pem")
    .h2()
    .host("0.0.0.0")
    .port(8443)
    .build()
    .run()
    .await
    .unwrap();
```

## Test the server

For a locally trusted or production certificate:

```sh
curl --http2 https://localhost:8443/
```

Use `-k` only for local testing with a self-signed certificate.

Protect private keys, renew certificates before expiration, and prefer a reverse proxy when your deployment platform already manages TLS.
