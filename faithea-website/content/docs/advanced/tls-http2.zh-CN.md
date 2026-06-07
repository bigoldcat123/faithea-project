---
title: TLS 与 HTTP/2
description: 使用证书和私钥提供 HTTPS，并启用 HTTP/2。
---

Faithea 可以直接终止 TLS，并通过 ALPN 协商 HTTP/2。

## 配置 TLS

提供 PEM 格式的私钥和证书：

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

调用 `.tls()` 会将默认监听地址改为 `0.0.0.0:443`。需要其他地址时，在它之后调用 `.host()` 或 `.port()`。

## 启用 HTTP/2

添加 `.h2()`，同时声明支持 HTTP/2 和 HTTP/1.1：

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

## 测试服务

使用本地信任或生产证书时：

```sh
curl --http2 https://localhost:8443/
```

只有在使用自签名证书进行本地测试时，才应使用 `-k`。

请保护私钥，在证书过期前完成续期；如果部署平台已经管理 TLS，优先使用反向代理。
