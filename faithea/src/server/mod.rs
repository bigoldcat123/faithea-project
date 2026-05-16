pub mod builder;
mod http1;
mod http2;
use std::{error::Error, future::poll_fn, pin::Pin, sync::Arc};

use bytes::{BufMut, BytesMut};
pub use faithea_io_core::BytesSource;
use hyper::body::{Body, Incoming};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    server::{
        builder::{GlobalErrorHandler, HttpServerBuilder},
        http1::H1Server,
        http2::H2Server,
    },
};

pub type HandlerModifier = Box<dyn Fn(&mut HandlerTire, &str)>;

pub enum Server {
    H1Server(H1Server),
    H2Server(H2Server),
    // O(HttpServer)
}
#[derive(Clone)]
pub struct ServerFuncProvider {
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
    error_handler: Option<Arc<GlobalErrorHandler>>,
}
impl ServerFuncProvider {
    pub(crate) fn new(
        handlers: Arc<HandlerTire>,
        guards: Arc<GuardTire>,
        error_handler: Option<Arc<GlobalErrorHandler>>,
    ) -> Self {
        Self {
            handlers,
            guards,
            error_handler,
        }
    }
    pub(crate) fn handlers(&self) -> Arc<HandlerTire> {
        self.handlers.clone()
    }
    pub(crate) fn guards(&self) -> Arc<GuardTire> {
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

pub struct HyperIncommingBytesSource {
    inner: Incoming,
    is_end: bool,
}
impl HyperIncommingBytesSource {
    pub(crate) fn new(incomming: Incoming) -> Self {
        HyperIncommingBytesSource {
            inner: incomming,
            is_end: false,
        }
    }
}

impl BytesSource for HyperIncommingBytesSource {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            if let Some(frame) = poll_fn(|cx| Pin::new(&mut self.inner).poll_frame(cx)).await {
                if let Ok(frame) = frame {
                    let data = frame
                        .into_data()
                        .map_err(|_| "frame.into_data error".to_string())?;
                    let len = data.len();
                    buf.put(data);
                    Ok(len)
                } else {
                    return Err("(0)    ".into());
                }
            } else {
                self.is_end = true;
                return Ok(0);
            }
        })
    }
    fn is_end(&self) -> bool {
        self.is_end
    }
}

pub(crate) struct Http1BytesSource<SOURCE: AsyncRead + Unpin> {
    source: SOURCE,
}
impl<SOURCE: AsyncRead + Unpin> Http1BytesSource<SOURCE> {
    pub(crate) fn stream(source: SOURCE) -> Self {
        Self { source }
    }
}
impl<SOURCE: AsyncRead + Unpin + Send> BytesSource for Http1BytesSource<SOURCE> {
    fn read_buf2<'a>(
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
