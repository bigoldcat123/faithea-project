---
title: TLS & HTTP/2
description: Serve HTTPS and enable HTTP/2 with a certificate and private key.
---

Faithea can terminate TLS directly and negotiate HTTP/2 using ALPN.

## Generate a self-signed certificate

The following OpenSSL commands generate a private key and self-signed certificate for local testing only. In production, use a certificate issued by a trusted CA or terminate TLS at a reverse proxy.

Create a directory for the certificate files:

```sh
mkdir -p certs
```

Generate a 2048-bit RSA private key:

```sh
openssl genpkey \
  -algorithm RSA \
  -pkeyopt rsa_keygen_bits:2048 \
  -out certs/key.pem
```

Use the private key to issue a self-signed certificate that is valid for 30 days:

```sh
openssl req -new -x509 \
  -sha256 \
  -days 30 \
  -key certs/key.pem \
  -out certs/cert.pem \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

The `subjectAltName` extension includes both `localhost` and `127.0.0.1`, allowing the service to be tested through either address.

Inspect the generated certificate:

```sh
openssl x509 \
  -in certs/cert.pem \
  -noout \
  -subject \
  -issuer \
  -dates \
  -ext subjectAltName
```

Do not commit private keys to Git. You can ignore the local test certificate directory:

```gitignore
certs/
```

## Configure TLS

Pass the PEM private key and certificate to `.tls()`. The first argument is the private key path, and the second is the certificate path.

```rust
use faithea::{get, handlers, server::HttpServer};

#[get("/")]
async fn index() {
    "secure Faithea server"
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(index))
        .tls("./certs/key.pem", "./certs/cert.pem")
        .host("127.0.0.1")
        .port(8443)
        .build()
        .run()
        .await
        .unwrap();
}
```

Calling `.tls()` changes the default listener to `0.0.0.0:443`. During local development, call `.host()` and `.port()` afterward to avoid binding a privileged port.

## Test HTTPS

Because the example uses a self-signed certificate, curl does not trust it by default. Use `-k` only during local testing to skip certificate verification:

```sh
curl -ik https://localhost:8443/
```

You can also explicitly trust the generated certificate:

```sh
curl -i \
  --cacert ./certs/cert.pem \
  https://localhost:8443/
```

Never disable certificate verification in production.

## Enable HTTP/2

Add `.h2()` to advertise HTTP/2 and HTTP/1.1 through TLS ALPN:

```rust
HttpServer::builder()
    .mount("/", handlers!(index))
    .tls("./certs/key.pem", "./certs/cert.pem")
    .h2()
    .host("127.0.0.1")
    .port(8443)
    .build()
    .run()
    .await
    .unwrap();
```

## Test HTTP/2

Confirm that the installed curl supports HTTP/2:

```sh
curl --version
```

The `Features` output should include `HTTP2`. Then send an HTTP/2 request:

```sh
curl -ik --http2 https://localhost:8443/
```

Use `-v` to inspect the TLS handshake and ALPN negotiation:

```sh
curl -kv --http2 https://localhost:8443/
```

An `HTTP/2` response indicates that protocol negotiation succeeded.

Protect private keys, renew certificates before expiration, and prefer a reverse proxy when your deployment platform already manages TLS.
