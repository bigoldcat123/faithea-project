use tokio::sync::mpsc::{Receiver, Sender};

use crate::websocket::data::WebSocketPayload;

pub struct WebSocket {
    sender: Sender<WebSocketPayload>,
    receiver: Receiver<WebSocketPayload>,
}

impl WebSocket {
    pub fn new(sender: Sender<WebSocketPayload>, receiver: Receiver<WebSocketPayload>) -> Self {
        Self { sender, receiver }
    }
    pub fn split(self) -> (Receiver<WebSocketPayload>, Sender<WebSocketPayload>) {
        (self.receiver, self.sender)
    }
}
