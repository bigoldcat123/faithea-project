//! A small, transport-agnostic server-side WebSocket protocol layer.
//!
//! This crate intentionally does **not** implement HTTP upgrade handshakes.
//! Give it a byte source that is already positioned at WebSocket frames, pass
//! it the outgoing message channel from your application, and it will parse
//! incoming server-side WebSocket messages while queueing automatic pong/close
//! replies onto that same outgoing channel.
//!
//! The original channel-oriented design is kept:
//!
//! ```no_run
//! # async fn demo<S>(source: S, outgoing_tx: tokio::sync::mpsc::Sender<websocket::WebSocketDataPayLoad>)
//! # where
//! #     S: websocket::BytesSource + 'static,
//! # {
//! let (parser, mut incoming_rx) =
//!     websocket::WebSocketIncommingMessageParser::new(source, outgoing_tx);
//! parser.start();
//!
//! while let Some(message) = incoming_rx.recv().await {
//!     println!("{:?}: {:?}", message.message_type(), message.as_bytes());
//! }
//! # }
//! ```

use std::{future::Future, pin::Pin};

use bytes::BytesMut;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    sync::mpsc,
};

pub mod websocket;

pub use websocket::{
    ProtocolError, WebSocketIncomingMessageParser, WebSocketIncommingMessageParser,
    WebSocketMessageType, data::WebSocketDataPayLoad, socket::WebSocket,
};

/// Async byte source used by the WebSocket frame parser.
///
/// The source may be a TCP stream, an in-memory stream, a TLS stream, or any
/// custom transport. It only needs to append newly-read bytes to the provided
/// buffer.
pub trait BytesSource: Send {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>>;

    fn is_end(&self) -> bool;
}

impl<T: AsyncRead + Send + Unpin> BytesSource for T {
    fn is_end(&self) -> bool {
        false
    }

    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move { self.read_buf(buf).await.map_err(|err| err.to_string()) })
    }
}

impl<'b> BytesSource for Box<dyn BytesSource + 'b> {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        self.as_mut().read_buf2(buf)
    }

    fn is_end(&self) -> bool {
        self.as_ref().is_end()
    }
}

impl<'b> BytesSource for Box<dyn BytesSource + Send + Sync + Unpin + 'b> {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        self.as_mut().read_buf2(buf)
    }

    fn is_end(&self) -> bool {
        self.as_ref().is_end()
    }
}

/// Async byte sink used to serialize outgoing WebSocket server frames.
///
/// Server frames are written unmasked, as required by RFC 6455.
pub trait BytesSink: Send {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
}

impl<T: AsyncWrite + Send + Unpin> BytesSink for T {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move { self.write_all(buf).await.map_err(|err| err.to_string()) })
    }

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move { self.flush().await.map_err(|err| err.to_string()) })
    }
}

impl<'b> BytesSink for Box<dyn BytesSink + 'b> {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.as_mut().write_all2(buf)
    }

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        self.as_mut().flush2()
    }
}

pub struct Handle {
    pub outcomming_message_receiver: mpsc::Receiver<WebSocketDataPayLoad>,
    pub websocket: WebSocket,
}

impl Handle {
    fn new(
        outcomming_message_receiver: mpsc::Receiver<WebSocketDataPayLoad>,
        outcomming_message_sender: mpsc::Sender<WebSocketDataPayLoad>,
        incoming_message_receiver: mpsc::Receiver<WebSocketDataPayLoad>,
    ) -> Self {
        Self {
            outcomming_message_receiver,
            websocket: WebSocket::new(outcomming_message_sender, incoming_message_receiver),
        }
    }
}

pub fn start_websocket<IO: BytesSource + 'static>(source: IO) -> Handle {
    let (outcomming_message_sender, outcomming_message_receiver) = mpsc::channel(10);
    let (p, incoming_receiver) =
        WebSocketIncommingMessageParser::new(source, outcomming_message_sender.clone());
    p.start();
    Handle::new(
        outcomming_message_receiver,
        outcomming_message_sender,
        incoming_receiver,
    )
}
