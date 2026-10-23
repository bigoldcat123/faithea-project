pub mod builder;
mod http1;
mod http2;

#[cfg(test)]
mod tests;

use std::{error::Error, future::poll_fn, pin::Pin, sync::Arc};

use bytes::{BufMut, BytesMut};
pub use faithea_io_core::BytesSource;
use hyper::body::{Body, Incoming};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    guard::GuardTrie,
    handler::HandlerTrie,
    server::{
        builder::{GlobalErrorHandler, HttpServerBuilder},
        http1::H1Server,
        http2::H2Server,
    },
};

pub type HandlerModifier = Box<dyn Fn(&mut HandlerTrie, &str)>;

pub enum Server {
    H1Server(H1Server),
    H2Server(H2Server),
    // O(HttpServer)
}
#[derive(Clone)]
pub struct ServerContext {
    handlers: Arc<HandlerTrie>,
    guards: Arc<GuardTrie>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
}
impl ServerContext {
    pub(crate) fn new(
        handlers: Arc<HandlerTrie>,
        guards: Arc<GuardTrie>,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Self {
        Self {
            handlers,
            guards,
            error_handler,
        }
    }
    pub(crate) fn handlers(&self) -> Arc<HandlerTrie> {
        self.handlers.clone()
    }
    pub(crate) fn guards(&self) -> Arc<GuardTrie> {
        self.guards.clone()
    }
    pub(crate) fn error_handler(&self) -> Option<Arc<GlobalErrorHandler>> {
        self.error_handler.clone()
    }
}

impl Server {
    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        match self {
            Server::H1Server(server) => server.run().await,
            // Server::O(server) => server.run().await,
            Server::H2Server(server) => server.run().await,
        }
    }
}

pub struct HttpServer;

impl HttpServer {
    pub fn builder() -> HttpServerBuilder {
        Default::default()
    }
}

pub struct HyperIncomingBytesSource {
    inner: Incoming,
    is_end: bool,
}
impl HyperIncomingBytesSource {
    pub(crate) fn new(incoming: Incoming) -> Self {
        HyperIncomingBytesSource {
            inner: incoming,
            is_end: false,
        }
    }
}

impl BytesSource for HyperIncomingBytesSource {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            while let Some(frame) = poll_fn(|cx| Pin::new(&mut self.inner).poll_frame(cx)).await {
                if let Ok(frame) = frame {
                    let data = match frame.into_data() {
                        Ok(data) => data,
                        // Trailers carry metadata, not request payload bytes.
                        Err(frame) if frame.is_trailers() => continue,
                        Err(_) => return Err("frame.into_data error".to_string()),
                    };
                    // Empty DATA frames are valid and do not signal transport EOF.
                    if data.is_empty() {
                        continue;
                    }
                    let len = data.len();
                    buf.put(data);
                    return Ok(len);
                } else {
                    return Err("(0)    ".into());
                }
            }
            self.is_end = true;
            Ok(0)
        })
    }
    fn is_end(&self) -> bool {
        self.is_end
    }
}

impl BytesSource for &mut HyperIncomingBytesSource {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        (**self).read_bytes(buf)
    }

    fn is_end(&self) -> bool {
        (**self).is_end()
    }
}

pub(crate) struct Http1BytesSource<Source: AsyncRead + Unpin> {
    source: Source,
}
impl<Source: AsyncRead + Unpin> Http1BytesSource<Source> {
    pub(crate) fn stream(source: Source) -> Self {
        Self { source }
    }
}
impl<Source: AsyncRead + Unpin + Send> BytesSource for Http1BytesSource<Source> {
    fn read_bytes<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            let res = AsyncReadExt::read_buf(&mut self.source, buf)
                .await
                .map_err(|err| err.to_string())?;
            if res == 0 {
                return Err("EOF ERROR".to_string());
            }
            Ok(res)
        })
    }
    fn is_end(&self) -> bool {
        false
    }
}
