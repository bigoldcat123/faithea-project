---
title: TLS 与 HTTP/2
description: 使用证书和私钥提供 HTTPS，并启用 HTTP/2。
---

Faithea 可以直接终止 TLS，并通过 ALPN 协商 HTTP/2。

## 生成自签名证书

下面使用 OpenSSL 生成一个只用于本地测试的私钥和自签名证书。生产环境应使用受信任 CA 签发的证书，或将 TLS 交给反向代理处理。

先创建存放证书的目录：

```sh
mkdir -p certs
```

生成一把 2048 位 RSA 私钥：

```sh
openssl genpkey \
  -algorithm RSA \
  -pkeyopt rsa_keygen_bits:2048 \
  -out certs/key.pem
```

使用私钥签发有效期为 30 天的自签名证书：

```sh
openssl req -new -x509 \
  -sha256 \
  -days 30 \
  -key certs/key.pem \
  -out certs/cert.pem \
  -subj "/CN=localhost" \
  -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"
```

`subjectAltName` 同时包含 `localhost` 和 `127.0.0.1`，方便使用域名或本地 IP 测试服务。

检查生成的证书：

```sh
openssl x509 \
  -in certs/cert.pem \
  -noout \
  -subject \
  -issuer \
  -dates \
  -ext subjectAltName
```

私钥文件不应提交到 Git。可以将测试证书目录加入 `.gitignore`：

```gitignore
certs/
```

## 配置 TLS

将 PEM 格式的私钥和证书传给 `.tls()`。第一个参数是私钥路径，第二个参数是证书路径。

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

调用 `.tls()` 会将默认监听地址改为 `0.0.0.0:443`。本地开发通常在它之后调用 `.host()` 和 `.port()`，避免绑定特权端口。

## 测试 HTTPS

由于示例使用自签名证书，curl 默认不会信任它。本地测试时可以使用 `-k` 跳过证书校验：

```sh
curl -ik https://localhost:8443/
```

也可以显式信任刚生成的证书进行测试：

```sh
curl -i \
  --cacert ./certs/cert.pem \
  https://localhost:8443/
```

`-k` 只能用于本地测试，生产环境不应跳过证书校验。

## 启用 HTTP/2

添加 `.h2()`，通过 TLS ALPN 同时声明支持 HTTP/2 和 HTTP/1.1：

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

## 测试 HTTP/2

确认当前 curl 支持 HTTP/2：

```sh
curl --version
```

输出的 `Features` 中应包含 `HTTP2`。然后发送 HTTP/2 请求：

```sh
curl -ik --http2 https://localhost:8443/
```

使用 `-v` 可以查看 TLS 握手与 ALPN 协商结果：

```sh
curl -kv --http2 https://localhost:8443/
```

输出中出现 `HTTP/2` 表示协议协商成功。

请保护私钥，在证书过期前完成续期；如果部署平台已经管理 TLS，优先使用反向代理。
