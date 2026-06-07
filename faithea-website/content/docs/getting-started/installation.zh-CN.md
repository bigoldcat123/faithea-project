---
title: 安装
description: 安装 Faithea，并准备一个 Rust 项目。
---

Faithea 以 Rust crate 的形式发布，因此可以添加到任意 Cargo 项目中。

## 环境要求

安装 Faithea 前，请确保本地已经具备：

- 较新的稳定版 Rust 工具链
- Rust 包管理器 Cargo
- 用于应用入口的 Tokio 异步运行时

检查 Rust 和 Cargo 是否可用：

```sh
rustc --version
cargo --version
```

## 创建项目

创建一个新的 Rust 二进制项目，并进入项目目录：

```sh
cargo new hello-faithea
cd hello-faithea
```

## 添加 Faithea

使用 `cargo add` 安装 Faithea 和 Tokio：

```sh
cargo add faithea
cargo add tokio --features macros,rt
```

这会选择兼容的 Faithea 版本，并启用启动异步应用所需的 Tokio 功能。

## 手动安装

你也可以直接在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
faithea = "<latest>"
tokio = { version = "1", features = ["macros", "rt"] }
```

请将 `<latest>` 替换为你希望使用的 Faithea 版本。推荐使用 `cargo add faithea`，由 Cargo 自动选择当前兼容版本。

## 验证安装

运行 Cargo 类型检查，下载并编译依赖：

```sh
cargo check
```

如果命令成功完成，项目就已经准备好编写第一个 Faithea 服务了。
