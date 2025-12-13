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

use std::collections::HashMap;

use bytes::{BufMut, Bytes, BytesMut};

pub mod data;
pub mod guard;
pub mod handler;
pub mod request;
pub mod response;
pub mod route;
pub mod server;

#[macro_export]
macro_rules! map_str {
    () => {
        |x| format!("{}", x)
    };
}
// impl ConvertFromRefString<i32> for  &String {
//     fn convert(self) -> Result<i32,String> {
//         self.parse::<i32>().map_err(|_|"convert error!".to_string())
//     }
// }

#[macro_export]
macro_rules! impl_convert_from_ref_string {
    ($($t:ty),*) => {
        $(
            impl <'a> $crate::request::ConvertFromRefString<'a,$t> for  &String {
                fn convert(self) -> Result<$t,String> {
                    self.parse::<$t>().map_err(|_|format!("can not convert {} to {}",self,stringify!($t)))
                }
            }
        )*
    };
}



#[derive(Debug, Default)]
pub struct HttpHeader {
    headers: HashMap<String, String>,
}
impl HttpHeader {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }
    pub fn parse_new_header(&mut self, s: &str) -> Result<(), String> {
        let mut k_v = s.split(":");
        let k = k_v
            .next()
            .ok_or("no key".to_string())?
            .trim()
            .to_lowercase();
        let v = k_v
            .next()
            .ok_or("no value".to_string())?
            .trim()
            .to_lowercase();
        self.headers.insert(k, v);
        Ok(())
    }


    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }


    pub fn add<K: AsRef<str>,V:AsRef<str>>(&mut self, key: K, value: V) {
        self.headers
            .insert(key.as_ref().to_string(), value.as_ref().to_string());
    }
}

impl From<&HttpHeader> for Bytes {
    fn from(value: &HttpHeader) -> Self {
        let mut b = BytesMut::with_capacity(256);
        for (k, v) in value.headers.iter() {
            b.put(format!("{k}:{v}\r\n").as_bytes());
        }
        b.put("\r\n".as_bytes());

        b.freeze()
    }
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

#[cfg(test)]
mod test {

    use bytes::{Buf, Bytes};

    use crate::HttpHeader;

    #[test]
    fn into_bytes_test() {
        let mut header = HttpHeader::new();
        // some real HTTP headers
        header.add("Host", "example.com");
        header.add("User-Agent", "rust-test/0.1");
        header.add("Accept", "*/*");
        header.add("Connection", "close");

        let bytes: Bytes = (&header).into();
        let s = std::str::from_utf8(bytes.chunk()).unwrap();
        // Split into lines (ignore the final empty line caused by the trailing \r\n\r\n),
        // sort to avoid depending on HashMap iteration order, and compare.
        let mut got: Vec<&str> = s.split("\r\n").filter(|l| !l.is_empty()).collect();
        got.sort();
        let mut expected = vec![
            "Host:example.com",
            "User-Agent:rust-test/0.1",
            "Accept:*/*",
            "Connection:close",
        ];
        expected.sort();
        assert_eq!(got, expected);
        assert!(s.ends_with("\r\n\r\n"));
    }
}


#[macro_export]
macro_rules! res_modifiers {
    ($($e:expr),*) => {
        {
            let a:Vec<Box<dyn $crate::response::HttpResponseModifier + Send + Sync>> = vec![
               $( Box::new($e),)*
            ];
            a
        }

    };
}


#[cfg(test)]
mod tests {


    use crate::response::{ResponseStatusLine};

    use super::*;

    #[test]
    fn macro_test() {
        let a = HttpHeader::new();
        let b = ResponseStatusLine::default();
        let _ = res_modifiers!(a,b);
    }
}
