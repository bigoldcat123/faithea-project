//! HTTP response building and serialization.
//!
//! This module provides data structures and methods for constructing HTTP responses
//! and serializing them to network sockets. It supports various response body types
//! including simple byte data, file streams, and empty responses.
//!
//! # Features
//!
//! - **Response Building**: Create responses with status codes, headers, and bodies
//! - **Multiple Body Types**: Support for bytes, files, and empty responses
//! - **Async Serialization**: Efficient async writing to network sockets
//! - **HTTP Compliance**: Proper HTTP formatting with status lines and headers
//!
//! # Examples
//!
//! ```rust
//! use http_server::response::{HttpResponse, ResponseBody};
//! use bytes::Bytes;
//!
//! // Create a simple text response
//! let mut response = HttpResponse::new();
//! response.add_header(("Content-Type", "text/plain"));
//! response.set_body(ResponseBody::Simple(Bytes::from("Hello, World!")));
//!
//! // Create a 404 Not Found response
//! let not_found = HttpResponse::not_found();
//! ```

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::{fs::File, io::AsyncWriteExt, net::tcp::OwnedWriteHalf};

use crate::HttpHeader;

/// Represents a complete HTTP response ready for serialization to a client.
///
/// This structure contains all components of an HTTP response: the status line
/// (protocol version, status code, reason phrase), headers, and body.
/// Responses can be constructed programmatically and then serialized to
/// a network socket using [`serialize_to_socket`](HttpResponse::serialize_to_socket).
///
/// # Fields
///
/// - `status_line`: The HTTP status line (version, status code, reason phrase)
/// - `headers`: HTTP headers as key-value pairs
/// - `body`: The response body content (bytes, file, or empty)
///
/// # Examples
///
/// ```rust
/// use http_server::response::{HttpResponse, ResponseBody};
/// use bytes::Bytes;
///
/// let mut response = HttpResponse::new();
/// response.add_header(("Content-Type", "application/json"));
/// response.set_body(ResponseBody::Simple(Bytes::from(r#"{"message": "OK"}"#)));
/// ```
#[derive(Default, Debug)]
pub struct HttpResponse {
    status_line: ResponseStatusLine,
    headers: HttpHeader,
    pub body: ResponseBody,
}
impl HttpResponse {
    /// Creates a new HTTP response with default values.
    ///
    /// The default response has:
    /// - Status line: `HTTP/1.1 200 OK`
    /// - No headers
    /// - Empty body
    ///
    /// # Returns
    ///
    /// A new `HttpResponse` instance with default values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::HttpResponse;
    ///
    /// let response = HttpResponse::new();
    /// // response has status 200 OK and empty body
    /// ```
    pub fn new() -> Self {
        Self {
            status_line: ResponseStatusLine::new("HTTP/1.1", "200", "OK"),
            headers: Default::default(),
            body: ResponseBody::Empty,
        }
    }
    /// Creates a standard 404 Not Found response.
    ///
    /// The response includes:
    /// - Status line: `HTTP/1.1 404 Not Found`
    /// - `Content-Length: 9` header
    /// - Body: `"not found"` (9 bytes)
    ///
    /// # Returns
    ///
    /// A fully configured 404 Not Found response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::HttpResponse;
    ///
    /// let not_found = HttpResponse::not_found();
    /// // Ready to send to client for unknown routes
    /// ```
    pub fn not_found() -> Self {
        let mut r = Self::new();
        r.status_line.status = "404".to_string();
        r.status_line.info = "Not Found".to_string();
        r.headers.add("Content-length", "9");
        r.set_body(ResponseBody::Simple("not found".into()));
        r
    }
    pub fn error(err_message:String) -> Self {
        let mut r = Self::new();
        r.status_line.status = "500".to_string();
        r.status_line.info = "error!".to_string();
        let body:Bytes = err_message.into();
        r.headers.add("Content-length", body.len().to_string());
        r.set_body(ResponseBody::Simple(body));
        r
    }
    /// Sets the response body content.
    ///
    /// This method replaces the current body with the provided value.
    /// The body can be simple bytes, a file handle, or empty.
    ///
    /// # Arguments
    ///
    /// * `body` - The new response body
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::{HttpResponse, ResponseBody};
    /// use bytes::Bytes;
    ///
    /// let mut response = HttpResponse::new();
    /// response.set_body(ResponseBody::Simple(Bytes::from("Hello")));
    /// ```
    pub fn set_body(&mut self, body: ResponseBody) {
        self.body = body
    }
    /// Adds an HTTP header to the response.
    ///
    /// If a header with the same name already exists, it will be replaced.
    /// Header names are case-insensitive when stored.
    ///
    /// # Arguments
    ///
    /// * `header_k_v` - A tuple of (header_name, header_value)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::HttpResponse;
    ///
    /// let mut response = HttpResponse::new();
    /// response.add_header(("Content-Type", "text/html"));
    /// response.add_header(("Cache-Control", "no-cache"));
    /// ```
    pub fn add_header<S: AsRef<str>>(&mut self, header_k_v: (S, S)) {
        self.headers.add(header_k_v.0, header_k_v.1);
    }
    /// Serializes the status line and headers to bytes.
    ///
    /// This method converts the HTTP status line and all headers to their
    /// wire format as a `Bytes` object. The body is not included in this output.
    ///
    /// The format is: `HTTP/1.1 200 OK\r\nHeader1: Value1\r\nHeader2: Value2\r\n\r\n`
    ///
    /// # Returns
    ///
    /// A `Bytes` object containing the serialized status line and headers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::HttpResponse;
    ///
    /// let mut response = HttpResponse::new();
    /// response.add_header(("Content-Type", "text/plain"));
    /// let bytes = response.line_header_bytes();
    /// assert!(bytes.len() > 0);
    /// ```
    pub fn line_header_bytes(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        let line_bytes: Bytes = (&self.status_line).into();
        bytes.put(line_bytes);
        let header_bytes: Bytes = (&self.headers).into();
        bytes.put(header_bytes);
        bytes.freeze()
    }
    /// Serializes and writes the complete HTTP response to a socket.
    ///
    /// This method writes the status line, headers, and body to the provided
    /// socket in the correct HTTP format. The response is consumed in the process.
    ///
    /// # Arguments
    ///
    /// * `socket` - The write half of a TCP stream to write the response to
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_server::response::{HttpResponse, ResponseBody};
    /// use bytes::Bytes;
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let socket = TcpStream::connect("127.0.0.1:8080").await?;
    /// let (_, mut writer) = socket.into_split();
    ///
    /// let mut response = HttpResponse::new();
    /// response.set_body(ResponseBody::Simple(Bytes::from("Hello")));
    /// response.serialize_to_socket(&mut writer).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serialize_to_socket(self, socket: &mut OwnedWriteHalf) {
        use self::ResponseBody::*;
        let mut line_header: Bytes = self.line_header_bytes();
        while line_header.has_remaining() {
            let _ = socket.write_buf(&mut line_header).await;
        }
        match self.body {
            Simple(mut b) => {
                let _ = socket.write_all_buf(&mut b).await;
            }
            File(mut f) => {
                let _ = tokio::io::copy(&mut f, socket).await;
            }
            Empty => {
                // No body to write
            }
        }
        let _ = socket.flush().await;
    }
}

/// Represents the body of an HTTP response.
///
/// This enum supports three types of response bodies:
/// 1. **Simple bytes**: In-memory byte data (for small responses)
/// 2. **File**: A file handle for streaming large files efficiently
/// 3. **Empty**: No body (for responses like 204 No Content)
///
/// # Variants
///
/// - `Simple(Bytes)`: In-memory byte data
/// - `File(File)`: A file handle for streaming
/// - `Empty`: No body content
///
/// # Examples
///
/// ```rust
/// use http_server::response::ResponseBody;
/// use bytes::Bytes;
/// use tokio::fs::File;
///
/// // Simple byte response
/// let body1 = ResponseBody::Simple(Bytes::from("Hello"));
///
/// // File response (in real usage, you'd open a file)
/// // let file = File::open("data.txt").await.unwrap();
/// // let body2 = ResponseBody::File(file);
///
/// // Empty response
/// let body3 = ResponseBody::Empty;
/// ```
#[derive(Default, Debug)]
pub enum ResponseBody {
    /// In-memory byte data for small responses.
    Simple(Bytes),
    /// File handle for streaming large responses efficiently.
    File(File),
    /// No body content.
    #[default]
    Empty,
}

/// Represents the status line of an HTTP response.
///
/// The status line consists of three parts:
/// 1. Protocol version (e.g., "HTTP/1.1")
/// 2. Status code (e.g., "200", "404", "500")
/// 3. Reason phrase (e.g., "OK", "Not Found", "Internal Server Error")
///
/// # Fields
///
/// - `version`: HTTP protocol version
/// - `status`: HTTP status code as a string
/// - `info`: Human-readable reason phrase
///
/// # Examples
///
/// ```rust
/// use http_server::response::ResponseStatusLine;
///
/// let status_line = ResponseStatusLine::new("HTTP/1.1", "200", "OK");
/// assert_eq!(status_line.version, "HTTP/1.1");
/// assert_eq!(status_line.status, "200");
/// assert_eq!(status_line.info, "OK");
/// ```
#[derive(Default, Debug)]
pub struct ResponseStatusLine {
    version: String,
    status: String,
    info: String,
}

impl ResponseStatusLine {
    /// Creates a new `ResponseStatusLine` with the specified components.
    ///
    /// # Arguments
    ///
    /// * `version` - HTTP protocol version (e.g., "HTTP/1.1")
    /// * `status` - HTTP status code as a string (e.g., "200", "404")
    /// * `info` - Human-readable reason phrase (e.g., "OK", "Not Found")
    ///
    /// # Returns
    ///
    /// A new `ResponseStatusLine` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::response::ResponseStatusLine;
    ///
    /// let status_line = ResponseStatusLine::new("HTTP/1.1", "404", "Not Found");
    /// assert_eq!(status_line.status, "404");
    /// ```
    pub fn new(version: &str, status: &str, info: &str) -> Self {
        Self {
            version: version.into(),
            status: status.into(),
            info: info.into(),
        }
    }
}

impl From<&ResponseStatusLine> for Bytes {
    /// Converts a `ResponseStatusLine` to its wire format as `Bytes`.
    ///
    /// The format is: `{version} {status} {info}\r\n`
    /// Example: `HTTP/1.1 200 OK\r\n`
    ///
    /// # Arguments
    ///
    /// * `value` - The status line to convert
    ///
    /// # Returns
    ///
    /// A `Bytes` object containing the serialized status line.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bytes::Bytes;
    /// use http_server::response::ResponseStatusLine;
    ///
    /// let status_line = ResponseStatusLine::new("HTTP/1.1", "200", "OK");
    /// let bytes: Bytes = (&status_line).into();
    /// assert_eq!(bytes.as_ref(), b"HTTP/1.1 200 OK\r\n");
    /// ```
    fn from(value: &ResponseStatusLine) -> Self {
        let mut bytes = BytesMut::with_capacity(64);
        bytes.put(format!("{} {} {}\r\n", value.version, value.status, value.info).as_bytes());
        bytes.freeze()
    }
}

pub trait HttpResponseModifier {
    fn modify(&self, res: &mut HttpResponse) -> Result<(), String>;
}

impl HttpResponseModifier for HttpHeader {
    fn modify(&self, res: &mut HttpResponse) -> Result<(), String> {
        for kv in self.headers.iter() {
            res.add_header(kv);
        }
        Ok(())
    }
}
impl HttpResponseModifier for ResponseStatusLine {
    fn modify(&self, res: &mut HttpResponse) -> Result<(), String> {
        res.status_line.info = self.status.to_string();
        res.status_line.info = self.info.to_string();
        Ok(())
    }
}
impl<T: HttpResponseModifier + ?Sized> HttpResponseModifier for Vec<Box<T>> {
    fn modify(&self, res: &mut HttpResponse) -> Result<(),String> {
        for m in self {
            m.modify(res)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::{Buf, Bytes};

    use crate::response::{HttpResponse, ResponseStatusLine};

    /// Tests conversion of `ResponseStatusLine` to bytes.
    #[test]
    fn into_bytes_test() {
        let r = ResponseStatusLine::new("Http", "200", "ok");
        let b: Bytes = (&r).into();
        assert_eq!(b"Http 200 ok\r\n", b.chunk());
    }

    /// Tests serialization of status line and headers to bytes.
    #[test]
    fn line_header_test() {
        let mut r = HttpResponse::new();
        r.add_header(("Hello", "World"));
        let b = r.line_header_bytes();
        println!("{:?}", b);
    }
}
