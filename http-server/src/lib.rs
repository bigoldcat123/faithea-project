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

/// Converts any value to a string using its `Display` implementation.
///
/// This macro creates a closure that formats any type implementing `Display`
/// into a `String`. It's commonly used as an error converter in parsing functions.
///
/// # Example
/// ```
/// use http_server::map_str;
///
/// let to_string = map_str!();
/// let result = to_string(42);
/// assert_eq!(result, "42");
/// ```
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
                    self.parse::<$t>().map_err(|_|"convert error!".to_string())
                }
            }
        )*
    };
}



/// Represents HTTP headers as a key-value store.
///
/// This structure stores HTTP headers in a case-insensitive manner (all keys
/// are converted to lowercase during parsing). It supports serialization to
/// the wire format and provides methods for header manipulation.
///
/// # Examples
/// ```
/// use http_server::HttpHeader;
///
/// let mut headers = HttpHeader::new();
/// headers.add("Content-Type", "application/json");
/// assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));
/// ```
#[derive(Debug, Default)]
pub struct HttpHeader {
    headers: HashMap<String, String>,
}
impl HttpHeader {
    /// Creates a new empty `HttpHeader`.
    ///
    /// # Returns
    /// A new `HttpHeader` instance with no headers.
    ///
    /// # Example
    /// ```
    /// use http_server::HttpHeader;
    ///
    /// let headers = HttpHeader::new();
    /// assert!(headers.get("content-type").is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }
    /// Parses a single HTTP header line and adds it to the collection.
    ///
    /// The header line should be in the format "Key: Value". Both key and value
    /// are trimmed of whitespace, and the key is converted to lowercase for
    /// case-insensitive lookups.
    ///
    /// # Arguments
    /// * `s` - A string slice containing the header line to parse
    ///
    /// # Returns
    /// * `Ok(())` if the header was successfully parsed and added
    /// * `Err(String)` if the header line is malformed
    ///
    /// # Example
    /// ```
    /// use http_server::HttpHeader;
    ///
    /// let mut headers = HttpHeader::new();
    /// headers.parse_new_header("Content-Type: application/json").unwrap();
    /// assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));
    /// ```
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
    /// Retrieves the value of a header by its key.
    ///
    /// The lookup is case-insensitive as all keys are stored in lowercase.
    /// If you want case-sensitive lookup, you should use the internal
    /// `headers` field directly.
    ///
    /// # Arguments
    /// * `key` - The header name to look up (case-insensitive)
    ///
    /// # Returns
    /// * `Some(&String)` if the header exists
    /// * `None` if the header doesn't exist
    ///
    /// # Example
    /// ```
    /// use http_server::HttpHeader;
    ///
    /// let mut headers = HttpHeader::new();
    /// headers.add("Content-Type", "text/html");
    /// assert_eq!(headers.get("content-type"), Some(&"text/html".to_string()));
    /// assert_eq!(headers.get("CONTENT-TYPE"), Some(&"text/html".to_string()));
    /// ```
    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    /// Adds a header to the collection.
    ///
    /// This is an internal method used for testing and should not be part of
    /// the public API. Headers should be added via `parse_new_header` to
    /// ensure consistent handling of case and whitespace.
    ///
    /// # Arguments
    /// * `key` - The header name
    /// * `value` - The header value
    pub fn add<K: AsRef<str>,V:AsRef<str>>(&mut self, key: K, value: V) {
        self.headers
            .insert(key.as_ref().to_string(), value.as_ref().to_string());
    }
}

impl From<&HttpHeader> for Bytes {
    /// Converts an `HttpHeader` to its wire format as `Bytes`.
    ///
    /// Serializes all headers in the format "Key: Value\r\n" followed by
    /// a terminating "\r\n". The headers are iterated in an unspecified
    /// order (determined by `HashMap` iteration order).
    ///
    /// # Example
    /// ```
    /// use bytes::Bytes;
    /// use http_server::HttpHeader;
    ///
    /// let mut headers = HttpHeader::new();
    /// headers.add("Host", "example.com");
    /// headers.add("Accept", "*/*");
    /// let bytes: Bytes = (&headers).into();
    /// let output = String::from_utf8_lossy(&bytes);
    /// assert!(output.contains("Host:example.com\r\n"));
    /// assert!(output.ends_with("\r\n\r\n"));
    /// ```
    fn from(value: &HttpHeader) -> Self {
        let mut b = BytesMut::with_capacity(256);
        for (k, v) in value.headers.iter() {
            b.put(format!("{k}:{v}\r\n").as_bytes());
        }
        b.put("\r\n".as_bytes());

        b.freeze()
    }
}

/// Normalizes a URL path to a consistent format.
///
/// This function ensures URL paths follow consistent rules:
/// 1. Always starts with "/" (unless it's an empty string)
/// 2. Never ends with "/" unless it's the root path "/"
/// 3. Empty string becomes "/"
///
/// # Arguments
/// * `s` - The URL path to normalize (any type that implements `AsRef<str>`)
///
/// # Returns
/// A normalized URL path string.
///
/// # Examples
/// ```
/// use http_server::regulate_url_path;
///
/// assert_eq!(regulate_url_path(""), "/");
/// assert_eq!(regulate_url_path("hello"), "/hello");
/// assert_eq!(regulate_url_path("/world"), "/world");
/// assert_eq!(regulate_url_path("api/users/"), "/api/users");
/// assert_eq!(regulate_url_path("/"), "/");
/// ```
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
