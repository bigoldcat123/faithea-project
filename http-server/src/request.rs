//! HTTP request parsing and data structures.
//!
//! This module provides functionality for parsing HTTP requests from raw bytes
//! and representing them in structured form. It handles the complete HTTP
//! request format including the request line, headers, and optional body.
//!
//! # Features
//!
//! - **Complete HTTP parsing**: Parses request line, headers, and body
//! - **Async I/O**: Uses Tokio for non-blocking network reads
//! - **Buffer management**: Efficient byte buffer reuse for performance
//! - **Error handling**: Detailed error messages for malformed requests
//!
//! # Usage
//!
//! ```rust
//! use http_server::request::{parse_http_frame, HttpRequest};
//! use bytes::BytesMut;
//! use tokio::net::TcpStream;
//!
//! # async fn example() -> Result<(), String> {
//! let socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
//! let (mut reader, _) = socket.into_split();
//! let mut buf = BytesMut::with_capacity(4096);
//!
//! let request = parse_http_frame(&mut reader, &mut buf).await?;
//! println!("Method: {}, URL: {}", request.req_line.method, request.req_line.url);
//! # Ok(())
//! # }
//! ```

use std::{collections::HashMap};

use bytes::{Buf, Bytes, BytesMut};
use tokio::{io::AsyncReadExt, net::tcp::OwnedReadHalf};

use crate::{HttpHeader, impl_convert_from_ref_string, map_str, route::Route};

#[derive(Debug)]
pub struct PathParam {
    _inner: HashMap<String, String>,
}
impl PathParam {
    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&String> {
        self._inner.get(key.as_ref())
    }
    pub fn try_from_route(handler_route: &Route, incoming_route: &Route) -> Result<Self, String> {
        use crate::route::RouteComponent::*;
        let mut _inner = HashMap::new();
        if handler_route.r.len() != incoming_route.r.len() {
            return Err("route len not match!".to_string());
        }
        for cmp in handler_route.r.iter().zip(incoming_route.r.iter()) {
            match cmp {
                (PathParam(p), Exact(v)) => {
                    _inner.insert(p.to_string(), v.to_string());
                }
                _ => {}
            }
        }
        if _inner.is_empty() {
            Err("no path params".to_string())
        } else {
            Ok(Self { _inner })
        }
    }
}

/// Represents a complete HTTP request.
///
/// This structure contains all components of an HTTP request:
/// the request line (method, URL, version), headers, and optional body.
/// It is produced by parsing raw HTTP bytes from a network stream.
///
/// # Fields
///
/// - `req_line`: The HTTP request line (method, URL, protocol version)
/// - `headers`: HTTP headers as key-value pairs
/// - `body`: Optional request body (present for POST, PUT, etc.)
///
/// # Examples
///
/// ```rust
/// use http_server::request::{HttpRequest, HttpReqLine, HttpHeader};
/// use bytes::Bytes;
///
/// let req_line = HttpReqLine::parse("GET /index.html HTTP/1.1").unwrap();
/// let headers = HttpHeader::new();
/// let request = HttpRequest::new(req_line, headers, None);
///
/// assert_eq!(request.req_line.method, "GET");
/// assert_eq!(request.req_line.url, "/index.html");
/// ```
#[derive(Debug)]
pub struct HttpRequest {
    pub req_line: HttpReqLine,
    pub headers: HttpHeader,
    pub body: Option<Bytes>,
    pub path_param: Option<PathParam>,
}

impl HttpRequest {
    /// Creates a new `HttpRequest` from its components.
    ///
    /// This constructor is typically used internally by the parser.
    /// External code should obtain `HttpRequest` instances by parsing
    /// network data with [`parse_http_frame`].
    ///
    /// # Arguments
    ///
    /// * `req_line` - The parsed HTTP request line
    /// * `headers` - The parsed HTTP headers
    /// * `body` - Optional request body bytes
    ///
    /// # Returns
    ///
    /// A new `HttpRequest` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::request::{HttpRequest, HttpReqLine, HttpHeader};
    /// use bytes::Bytes;
    ///
    /// let req_line = HttpReqLine::parse("POST /api/data HTTP/1.1").unwrap();
    /// let headers = HttpHeader::new();
    /// let body = Some(Bytes::from("request payload"));
    /// let request = HttpRequest::new(req_line, headers, body);
    /// ```
    pub fn new(req_line: HttpReqLine, headers: HttpHeader, body: Option<Bytes>) -> Self {
        Self {
            req_line,
            headers,
            body,
            path_param: None,
        }
    }
    pub fn fake() -> Self {
        let req_line = HttpReqLine::parse("POST /api/data HTTP/1.1").unwrap();
        let headers = HttpHeader::new();
        let body = Some(Bytes::from("request payload"));
        HttpRequest::new(req_line, headers, body)
    }
    pub fn assamble_pathparam(&mut self, handler_route: &Route, incoming_route: &Route) {
        if let Ok(p) = PathParam::try_from_route(handler_route, incoming_route) {
            self.path_param = Some(p)
        }
    }
    pub fn get_pathparam<S: AsRef<str>>(&self, key: S) -> Option<&String> {
        if let Some(ref p) = self.path_param {
            p.get(key)
        } else {
            None
        }
    }
}

/// Represents the HTTP request line (first line of an HTTP request).
///
/// The request line contains three components separated by spaces:
/// 1. HTTP method (GET, POST, PUT, DELETE, etc.)
/// 2. Request URL/path
/// 3. HTTP protocol version
///
/// # Fields
///
/// - `method`: HTTP method (e.g., "GET", "POST")
/// - `url`: Request URL/path (e.g., "/index.html")
/// - `version`: HTTP version (e.g., "HTTP/1.1")
///
/// # Examples
///
/// ```rust
/// use http_server::request::HttpReqLine;
///
/// let req_line = HttpReqLine::parse("GET /api/users HTTP/1.1").unwrap();
/// assert_eq!(req_line.method, "GET");
/// assert_eq!(req_line.url, "/api/users");
/// assert_eq!(req_line.version, "HTTP/1.1");
/// ```
#[derive(Debug)]
pub struct HttpReqLine {
    pub method: String,
    pub url: String,
    pub version: String,
}

impl HttpReqLine {
    /// Parses an HTTP request line string into a structured `HttpReqLine`.
    ///
    /// The input should be a complete HTTP request line as received from
    /// the network, e.g., "GET /index.html HTTP/1.1".
    ///
    /// # Arguments
    ///
    /// * `s` - The HTTP request line string to parse
    ///
    /// # Returns
    ///
    /// * `Ok(HttpReqLine)` - Successfully parsed request line
    /// * `Err(String)` - Malformed request line with error description
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The string doesn't contain exactly three whitespace-separated tokens
    /// - Any required component (method, URL, version) is missing
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::request::HttpReqLine;
    ///
    /// let req_line = HttpReqLine::parse("POST /submit HTTP/1.1").unwrap();
    /// assert_eq!(req_line.method, "POST");
    /// assert_eq!(req_line.url, "/submit");
    /// assert_eq!(req_line.version, "HTTP/1.1");
    ///
    /// // Invalid request line
    /// let result = HttpReqLine::parse("GET /index.html");
    /// assert!(result.is_err());
    /// ```
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut head_line = s.split_whitespace();
        let method = head_line
            .next()
            .ok_or("method parsing error".to_string())?
            .to_string();
        let url = head_line
            .next()
            .ok_or("url parsing error".to_string())?
            .to_string();
        let version = head_line
            .next()
            .ok_or("version parsing error".to_string())?
            .to_string();
        Ok(Self {
            method,
            url,
            version,
        })
    }
}

/// Parses a complete HTTP request frame from a network stream.
///
/// This is the main entry point for HTTP request parsing. It reads from
/// the provided stream until a complete HTTP request (including optional body)
/// has been parsed, reusing the provided buffer for efficiency.
///
/// The parsing process:
/// 1. Reads until the complete request line and headers are received
/// 2. Parses the request line and headers into structured data
/// 3. If a "Content-Length" header is present, reads the specified number of body bytes
/// 4. Returns a complete `HttpRequest` structure
///
/// # Arguments
///
/// * `r` - The read half of a TCP stream to read HTTP data from
/// * `buf` - A reusable buffer for reading data (must be empty or contain
///   partial data from a previous read)
///
/// # Returns
///
/// * `Ok(HttpRequest)` - Successfully parsed HTTP request
/// * `Err(String)` - Error during parsing (malformed request, connection closed, etc.)
///
/// # Errors
///
/// Returns an error if:
/// - The connection is closed before a complete request is received
/// - The HTTP request is malformed (invalid format, missing components, etc.)
/// - The "Content-Length" value cannot be parsed as a number
/// - An I/O error occurs while reading from the stream
///
/// # Examples
///
/// ```rust,no_run
/// use http_server::request::parse_http_frame;
/// use bytes::BytesMut;
/// use tokio::net::TcpStream;
///
/// # async fn example() -> Result<(), String> {
/// let socket = TcpStream::connect("127.0.0.1:8080").await.unwrap();
/// let (mut reader, _) = socket.into_split();
/// let mut buf = BytesMut::with_capacity(4096);
///
/// let request = parse_http_frame(&mut reader, &mut buf).await?;
/// println!("Received {} request for {}", request.req_line.method, request.req_line.url);
/// # Ok(())
/// # }
/// ```
pub async fn parse_http_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<HttpRequest, String> {
    let (l, h) = parse_line_header_frame(r, buf).await?;
    let mut req = HttpRequest::new(l, h, None);
    if let Some(len) = req.headers.get("content-length") {
        let len = len.parse::<usize>().map_err(map_str!())?;
        parse_body_frame(len, r, buf).await?;
        let body = buf.split_to(len).freeze();
        req.body = Some(body);
    }
    Ok(req)
}

/// Reads the request body from the stream until the specified length is reached.
///
/// This function continues reading from the stream until the buffer contains
/// at least `len` bytes, which represents the complete request body.
/// It's used when a "Content-Length" header indicates a request body is present.
///
/// # Arguments
///
/// * `len` - The expected length of the request body in bytes
/// * `r` - The read half of a TCP stream to read from
/// * `buf` - Buffer to accumulate the body bytes into
///
/// # Returns
///
/// * `Ok(())` - Successfully read the complete body
/// * `Err(String)` - Connection closed before full body was read, or I/O error
///
/// # Note
///
/// This function assumes the buffer may already contain some data from
/// previous reads (e.g., the headers). It only reads additional bytes
/// as needed to reach the required length.
async fn parse_body_frame(
    len: usize,
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(), String> {
    loop {
        if buf.len() >= len {
            break;
        }
        if let Ok(len) = r.read_buf(buf).await {
            if len == 0 {
                return Err("other side closed".to_string());
            }
        } else {
            return Err("error!".to_string());
        }
    }
    Ok(())
}
/// Reads and parses the request line and headers from the stream.
///
/// This function reads from the stream until it detects the "\r\n\r\n"
/// sequence that marks the end of HTTP headers. It then parses the
/// accumulated data into a request line and header collection.
///
/// # Arguments
///
/// * `r` - The read half of a TCP stream to read from
/// * `buf` - Buffer to accumulate the header bytes
///
/// # Returns
///
/// * `Ok((HttpReqLine, HttpHeader))` - Successfully parsed request line and headers
/// * `Err(String)` - Connection closed before headers complete, or parse error
///
/// # Note
///
/// The buffer is advanced (consumed) after successful parsing to prepare
/// for reading the request body (if any).
async fn parse_line_header_frame(
    r: &mut OwnedReadHalf,
    buf: &mut BytesMut,
) -> Result<(HttpReqLine, HttpHeader), String> {
    loop {
        if let Ok(read_len) = r.read_buf(buf).await {
            if read_len == 0 {
                return Err("other side closed".to_string());
            }
            let (check_header_is_ok, position) = check_header(buf.chunk());
            if check_header_is_ok {
                let (l, h) = parse_line_header(&buf[..position])?;

                buf.advance(position);
                return Ok((l, h));
            }
        } else {
            return Err("error!".to_string());
        }
    }
}

/// Parses raw HTTP header bytes into a request line and header collection.
///
/// This function expects the complete HTTP headers (including the request line)
/// as a byte slice ending with "\r\n\r\n". It splits the data by lines,
/// parses the first line as the request line, and subsequent non-empty
/// lines as headers.
///
/// # Arguments
///
/// * `raw_header` - Byte slice containing the complete HTTP headers
///   (must end with "\r\n\r\n")
///
/// # Returns
///
/// * `Ok((HttpReqLine, HttpHeader))` - Successfully parsed components
/// * `Err(String)` - Invalid UTF-8, malformed request line, or malformed headers
///
/// # Errors
///
/// Returns an error if:
/// - The byte slice is not valid UTF-8
/// - The request line is missing or malformed
/// - Any header line doesn't contain the ":" separator
fn parse_line_header(raw_header: &[u8]) -> Result<(HttpReqLine, HttpHeader), String> {
    let raw_header = str::from_utf8(raw_header).map_err(map_str!())?;
    let mut raw_header = raw_header.split("\r\n");
    let req_line = HttpReqLine::parse(
        raw_header
            .next()
            .ok_or("parse req line error".to_string())?,
    )?;
    let mut http_header = HttpHeader::new();
    for h in raw_header {
        if !h.is_empty() {
            http_header.parse_new_header(h)?;
        }
    }
    Ok((req_line, http_header))
}

/// Checks if a byte slice contains the HTTP header terminator "\r\n\r\n".
///
/// This function scans the byte slice looking for the sequence "\r\n\r\n"
/// which marks the end of HTTP headers. It returns both whether the
/// terminator was found and its position (end index).
///
/// # Arguments
///
/// * `c` - Byte slice to search for the header terminator
///
/// # Returns
///
/// A tuple where:
/// - First element: `true` if "\r\n\r\n" was found, `false` otherwise
/// - Second element: If found, the index just past the terminator;
///   if not found, returns 0
///
/// # Note
///
/// The position returned is the index of the first byte *after* the
/// "\r\n\r\n" sequence, which is convenient for slicing.
fn check_header(c: &[u8]) -> (bool, usize) {
    for i in 0..=c.len() - 4 {
        if &c[i..i + 4] == b"\r\n\r\n" {
            return (true, i + 4);
        }
    }
    (false, 0)
}

pub trait ConvertFromRefString<'a,O> {
    fn convert(self) -> Result<O, String>;
}
impl_convert_from_ref_string!(
    i8, i16, i32, i64, i128, isize, usize, f32, f64, u8, u16, u32, u64, u128, bool
);

impl<'a,T> ConvertFromRefString<'a,T> for T {
    fn convert(self) -> Result<T, String> {
        Ok(self)
    }
}

impl <'a> ConvertFromRefString<'a,&'a str> for &'a String {
    fn convert(self) -> Result<&'a str, String> {
        Ok(self.as_str())
    }
}
impl <'a> ConvertFromRefString<'a,String> for &'a String {
    fn convert(self) -> Result<String, String> {
        Ok(self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_single_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}");
        let incoming_route = Route::from("/hello/chenzhonghai");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        assert_eq!(a, "chenzhonghai")
    }
    #[test]
    fn path_multi_param_parsing_test() {
        let handler_route = Route::from("/hello/{name}/{age}/dadigua");
        let incoming_route = Route::from("/hello/chenzhonghai/22/dadigua");
        let p = PathParam::try_from_route(&handler_route, &incoming_route).unwrap();
        let a = p.get("name").unwrap();
        let age = p.get("age").unwrap();
        assert_eq!(a, "chenzhonghai");
        assert_eq!(age, "22");
    }
}
