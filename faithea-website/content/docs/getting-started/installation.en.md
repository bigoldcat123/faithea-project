---
title: Installation
description: Install Faithea and prepare a Rust project.
---

Faithea is distributed as a Rust crate, so you can add it to any Cargo project.

## Requirements

Before installing Faithea, make sure you have:

- A recent stable Rust toolchain
- Cargo, Rust's package manager
- An async Tokio runtime for your application's entry point

Check that Rust and Cargo are available:

```sh
rustc --version
cargo --version
```

## Create a project

Create a new Rust binary project and enter its directory:

```sh
cargo new hello-faithea
cd hello-faithea
```

## Add Faithea

Use `cargo add` to install Faithea and Tokio:

```sh
cargo add faithea
cargo add tokio --features macros,rt
```

This selects a compatible Faithea release and enables the Tokio features needed to start an async application.

## Install manually

You can also add the dependencies directly to `Cargo.toml`:

```toml
[dependencies]
faithea = "<latest>"
tokio = { version = "1", features = ["macros", "rt"] }
```

Replace `<latest>` with the Faithea version you want to use. Using `cargo add faithea` is recommended because Cargo chooses the current compatible release for you.

## Verify the installation

Run Cargo's type checker to download and compile the dependencies:

```sh
cargo check
```

If the command finishes successfully, your project is ready for its first Faithea server.
