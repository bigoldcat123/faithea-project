---
title: 静态文件
description: 返回单个文件，或映射整个本地目录。
---

Faithea 可以从 handler 返回单个文件，也可以将 URL 模式映射到本地目录。

## 返回单个文件

使用 `StaticFile` 作为 handler 响应：

```rust
use faithea::{data::outbound::StaticFile, get};

#[get("/download")]
async fn download() {
    StaticFile("./public/manual.pdf")
}
```

Faithea 会异步读取文件，设置内容长度，并根据扩展名选择内容类型。

## 映射目录

在服务构建器上使用 `static_map`：

```rust
use faithea::server::HttpServer;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .static_map("/assets/**", "./public")
        .port(3000)
        .build()
        .run()
        .await
        .unwrap();
}
```

例如，请求 `/assets/icons/logo.svg` 时，会在 `./public` 中解析对应文件。

## 部署安全

只映射明确用于公开访问的目录。Secret、源码和生成的私有数据应放在映射目录之外。

请使用部署环境中稳定存在的路径，并通过全局错误处理器验证文件缺失时的行为。
