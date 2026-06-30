//! # HTTP Server Library
//!
//! A lightweight, asynchronous HTTP server framework built with Tokio.
//!
//! This library provides core components for building HTTP servers with support for:
//! - HTTP request/response parsing
//! - Routing with various pattern matching types
//! - Guard middleware system for request validation
//! - Asynchronous request handling
//!
//! ## Key Features
//! - **Async-first**: Built on Tokio for high-performance async I/O
//! - **Flexible Routing**: Supports exact, parameterized, and wildcard routes
//! - **Middleware Guards**: Chainable guards for authentication and validation
//! - **Type Safety**: Leverages Rust's type system for compile-time safety
//!
//! ## Modules
//! - `request`: HTTP request parsing and structures
//! - `response`: HTTP response building and serialization
//! - `server`: Main HTTP server implementation
//! - `handler`: Request handler and routing system
//! - `guard`: Guard middleware for request validation
//! - `route`: Route pattern matching components

pub mod data;
pub mod guard;
pub mod handler;
pub mod header;
mod proxy;
pub mod request;
pub mod response;
pub mod route;
pub mod server;
pub mod util;
pub use faithea_macro::*;
pub use http::HeaderMap;
pub use http::StatusCode;
pub mod error;
pub mod io;
pub mod service;
pub use faithea_websocket;
pub mod websocket {
    pub use faithea_websocket::*;

    pub mod data {
        pub use faithea_websocket::WebSocketDataPayLoad;
    }

    pub mod socket {
        pub use faithea_websocket::WebSocket;
    }
}

#[macro_export]
macro_rules! map_str {
    () => {
        |x| format!("{}", x)
    };
}

#[macro_export]
macro_rules! map_fu {
    () => {
        |_| $crate::error::Error::Unknown
    };
}
pub fn regulate_url_path<T: AsRef<str>>(s: T) -> String {
    let a: &str = s.as_ref();
    let mut v = a.into();
    if !a.starts_with("/") {
        v = format!("/{}", a);
    }
    if v.ends_with("/") && v.len() != 1 {
        v.pop();
    }
    v.to_string()
}

pub type ResponseModifier = Vec<Box<dyn crate::response::HttpResponseModifier + Send + Sync>>;

#[macro_export]
macro_rules! res_modifiers {
    ($($e:expr),* $(,)?) => {
        {
            let a:$crate::ResponseModifier = vec![
               $( Box::new($e),)*
            ];
            a
        }
    };
}
/// this is core trait for handler param analysis
pub trait TryConvertFrom<T>: Sized {
    type Error: std::error::Error;
    fn try_convert_from(value: T) -> Result<Self, Self::Error>;
}
/// please impl `TryConvertFrom`
pub trait TryConvertInto<O> {
    type Error: std::error::Error;
    fn try_convert_into(self) -> Result<O, Self::Error>;
}

impl<O, T: TryConvertFrom<O>> TryConvertInto<T> for O {
    type Error = T::Error;
    fn try_convert_into(self) -> Result<T, Self::Error> {
        T::try_convert_from(self)
    }
}

#[cfg(test)]
mod test {
    use http::{HeaderMap, StatusCode};
    #[test]
    fn macro_test() {
        let s = StatusCode::OK;
        let h = HeaderMap::new();
        let _ = res_modifiers!(s, h);
    }
    // use http::{HeaderMap, header::{ACCEPT, CONNECTION, HOST, USER_AGENT}};

    // #[test]
    // fn into_bytes_test() {
    //     let mut header = HeaderMap::new();
    //     // some real HTTP headers
    //     header.insert(HOST, "example.com".parse().unwrap());
    //     header.insert(USER_AGENT, "rust-test/0.1".parse().unwrap());
    //     header.insert(ACCEPT, "*/*".parse().unwrap());
    //     header.insert(CONNECTION, "close".parse().unwrap());

    //     let bytes: Bytes = (&header).into();
    //     let s = std::str::from_utf8(bytes.chunk()).unwrap();
    //     // Split into lines (ignore the final empty line caused by the trailing \r\n\r\n),
    //     // sort to avoid depending on HashMap iteration order, and compare.
    //     let mut got: Vec<&str> = s.split("\r\n").filter(|l| !l.is_empty()).collect();
    //     got.sort();
    //     let mut expected = vec![
    //         "Host:example.com",
    //         "User-Agent:rust-test/0.1",
    //         "Accept:*/*",
    //         "Connection:close",
    //     ];
    //     expected.sort();
    //     assert_eq!(got, expected);
    //     assert!(s.ends_with("\r\n\r\n"));
    // }
}
