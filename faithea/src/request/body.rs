use std::{future::Future, pin::Pin, time::Duration};

use bytes::BytesMut;
use http::{HeaderMap, HeaderValue, header::CONTENT_LENGTH};

use super::{RequestBody, content_type::ContentType};
use crate::{
    data::inbound::multipart::parser::h1::MultipartBodyParser, error::BodyParseError,
    server::BytesSource,
};

#[cfg(test)]
mod tests;

/// Per-request limits shared by HTTP/1 and HTTP/2. Sizes are in bytes.
/// The timeout covers the entire body, including multipart epilogues and trailers.
#[derive(Debug, Clone, Copy)]
pub struct BodyLimits {
    pub max_body_size: usize,
    pub max_file_size: usize,
    pub max_field_size: usize,
    /// Counts all multipart parts, including duplicate field names and files.
    pub max_parts: usize,
    pub max_part_header_size: usize,
    pub read_timeout: Duration,
}

impl Default for BodyLimits {
    fn default() -> Self {
        Self {
            max_body_size: 16 * 1024 * 1024,
            max_file_size: 8 * 1024 * 1024,
            max_field_size: 1024 * 1024,
            max_parts: 128,
            max_part_header_size: 16 * 1024,
            read_timeout: Duration::from_secs(30),
        }
    }
}

pub async fn parse_body_frame<Source: BytesSource>(
    source: Source,
    buf: &mut BytesMut,
    headers: &HeaderMap<HeaderValue>,
) -> Result<RequestBody, BodyParseError> {
    parse_body_frame_with_limits(source, buf, headers, BodyLimits::default()).await
}

/// Reads a body with bounded memory/disk usage and a deadline. Content-Length is
/// checked before reading; actual bytes are counted even for chunked bodies.
pub async fn parse_body_frame_with_limits<Source: BytesSource>(
    source: Source,
    buf: &mut BytesMut,
    headers: &HeaderMap<HeaderValue>,
    limits: BodyLimits,
) -> Result<RequestBody, BodyParseError> {
    if headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > limits.max_body_size as u64)
        || buf.len() > limits.max_body_size
    {
        return Err(body_size_error(limits.max_body_size));
    }
    let mut source = LimitedBodySource {
        inner: source,
        remaining: limits.max_body_size - buf.len(),
        exceeded: false,
        eof: false,
    };
    let result = tokio::time::timeout(limits.read_timeout, async {
        let body = match ContentType::try_from(headers)? {
            ContentType::MultipartFormData(boundary) => {
                MultipartBodyParser::parse_with_limits(&mut source, buf, boundary, limits).await?
            }
            _ => {
                while !source.is_end() {
                    if source.read_bytes(buf).await? == 0 {
                        break;
                    }
                }
                RequestBody::Simple(buf.split().freeze())
            }
        };
        // Multipart ends at its closing boundary. Drain its epilogue/trailers
        // under the same byte limit and deadline before reusing a connection.
        while !source.is_end() {
            buf.clear();
            source.read_bytes(buf).await?;
        }
        Ok(body)
    })
    .await;
    if source.exceeded {
        return Err(body_size_error(limits.max_body_size));
    }
    result.map_err(|_| BodyParseError::Timeout)?
}

fn body_size_error(limit: usize) -> BodyParseError {
    BodyParseError::LimitExceeded {
        resource: "request body",
        limit,
    }
}

struct LimitedBodySource<R> {
    inner: R,
    remaining: usize,
    exceeded: bool,
    eof: bool,
}

impl<R: BytesSource> BytesSource for LimitedBodySource<R> {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            let previous_len = buf.len();
            self.inner.read_bytes(buf).await?;
            let read = buf.len() - previous_len;
            if read > self.remaining {
                self.exceeded = true;
                buf.truncate(previous_len);
                return Err("request body size limit exceeded".into());
            }
            self.remaining -= read;
            self.eof = read == 0;
            Ok(read)
        })
    }

    fn is_end(&self) -> bool {
        self.eof || self.inner.is_end()
    }
}

impl<R: BytesSource> BytesSource for &mut LimitedBodySource<R> {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        (**self).read_bytes(buf)
    }

    fn is_end(&self) -> bool {
        (**self).is_end()
    }
}
