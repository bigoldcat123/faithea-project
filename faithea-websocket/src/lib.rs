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
//! # use faithea_websocket::{BytesSource, WebSocketPayload, WebSocketIncomingMessageParser};
//! # async fn demo<S>(source: S, outgoing_tx: tokio::sync::mpsc::Sender<WebSocketPayload>)
//! # where
//! #     S: BytesSource + 'static,
//! # {
//! let (parser, mut incoming_rx) =
//!     WebSocketIncomingMessageParser::new(source, outgoing_tx);
//! parser.start();
//!
//! while let Some(message) = incoming_rx.recv().await {
//!     println!("{:?}: {:?}", message.message_type(), message.as_bytes());
//! }
//! # }
//! ```

use tokio::sync::mpsc;

pub mod websocket;

pub use faithea_io_core::{BytesSink, BytesSource};
pub use websocket::{
    ProtocolError, WebSocketIncomingMessageParser, WebSocketMessageType, data::WebSocketPayload,
    socket::WebSocket,
};

pub struct WebSocketHandle {
    pub outgoing_message_receiver: mpsc::Receiver<WebSocketPayload>,
    pub websocket: WebSocket,
}

impl WebSocketHandle {
    fn new(
        outgoing_message_receiver: mpsc::Receiver<WebSocketPayload>,
        outgoing_message_sender: mpsc::Sender<WebSocketPayload>,
        incoming_message_receiver: mpsc::Receiver<WebSocketPayload>,
    ) -> Self {
        Self {
            outgoing_message_receiver,
            websocket: WebSocket::new(outgoing_message_sender, incoming_message_receiver),
        }
    }
}

pub fn start_websocket<Source: BytesSource + 'static>(source: Source) -> WebSocketHandle {
    let (outgoing_message_sender, outgoing_message_receiver) = mpsc::channel(10);
    let (parser, incoming_receiver) =
        WebSocketIncomingMessageParser::new(source, outgoing_message_sender.clone());
    parser.start();
    WebSocketHandle::new(
        outgoing_message_receiver,
        outgoing_message_sender,
        incoming_receiver,
    )
}
