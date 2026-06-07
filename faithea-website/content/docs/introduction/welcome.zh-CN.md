---
title: 欢迎
description: 简单了解 Faithea 及其核心设计理念。
---

Faithea 是一个基于 Tokio 构建的轻量级 Rust 异步 HTTP 框架。

它专注于提供小而易懂的 API，同时让 HTTP 开发中的重要部分保持明确。路由通过清晰的宏声明，请求数据被提取为 Rust 类型，响应则可以自由组合。

## 为什么选择 Faithea

Faithea 面向希望做到以下几点的开发者：

- 在没有多余机制的情况下构建异步 HTTP 服务。
- 保持路由与处理器代码紧凑、清晰。
- 使用普通 Rust 类型扩展请求和响应行为。
- 保持贴近 Rust 与 Tokio，而不是将它们隐藏起来。

## 一个简单示例

```rust
use faithea::{get, HttpServer};

#[get("/hello/{name}")]
async fn hello(name: String) {
    format!("Hello, {name}!")
}
```

本文档会随着框架持续完善。后续的 **Getting Started** 部分将介绍如何创建并运行一个小型服务。
