use tokio::sync::mpsc::{Receiver, Sender};

use crate::websocket::data::WebSocketDataPayLoad;

pub struct WebSocket {
    sender: Sender<WebSocketDataPayLoad>,
    reciver: Receiver<WebSocketDataPayLoad>,
}

impl WebSocket {
    pub fn new(
        sender: Sender<WebSocketDataPayLoad>,
        reciver: Receiver<WebSocketDataPayLoad>,
    ) -> Self {
        Self { sender, reciver }
    }
    pub fn split(self) -> (Receiver<WebSocketDataPayLoad>, Sender<WebSocketDataPayLoad>) {
        (self.reciver, self.sender)
    }
}
