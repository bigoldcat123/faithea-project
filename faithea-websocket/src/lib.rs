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
//! # use faithea_websocket::{BytesSource, WebSocketDataPayLoad, WebSocketIncommingMessageParser};
//! # async fn demo<S>(source: S, outgoing_tx: tokio::sync::mpsc::Sender<WebSocketDataPayLoad>)
//! # where
//! #     S: BytesSource + 'static,
//! # {
//! let (parser, mut incoming_rx) =
//!     WebSocketIncommingMessageParser::new(source, outgoing_tx);
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
    ProtocolError, WebSocketIncomingMessageParser, WebSocketIncommingMessageParser,
    WebSocketMessageType, data::WebSocketDataPayLoad, socket::WebSocket,
};

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
