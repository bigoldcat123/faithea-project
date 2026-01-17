use bytes::{Buf, BufMut, Bytes, BytesMut};
use h2::RecvStream;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::mpsc::{Receiver, Sender},
};

use crate::websocket::data::WebSocketDataPayLoad;

pub mod data;
pub mod socket;

#[derive(Debug,PartialEq, Eq)]
enum WebSocketActorState {
    Head,
    Body {
        len: usize,
        mask: [u8; 4],
        msg_finished: bool,
        readed: usize,
    },
    Finished,
    ConnectionClose,
}
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
enum WebSocketMessageType {
    Continuation, // %x0
    Text,         // %x1
    Binary,       // %x2
    // %x3-7 are reserved for further non-control frames
    Close, // %x8
    Ping,  // %x9
    Pong,  // %xA
           // %xB-F are reserved for further control frames
}
impl From<u8> for WebSocketMessageType {
    fn from(value: u8) -> Self {
        match value & 0x0F {
            // 取最后4位（操作码）
            0x0 => WebSocketMessageType::Continuation,
            0x1 => WebSocketMessageType::Text,
            0x2 => WebSocketMessageType::Binary,
            0x8 => WebSocketMessageType::Close,
            0x9 => WebSocketMessageType::Ping,
            0xA => WebSocketMessageType::Pong,
            _ => panic!("Invalid WebSocket opcode: 0x{:02X}", value),
        }
    }
}
impl From<WebSocketMessageType> for u8 {
    fn from(value: WebSocketMessageType) -> Self {
        use WebSocketMessageType::*;
        match value {
            Continuation => 0x0,
            Text => 0x1,
            Binary => 0x2,
            Close => 0x8,
            Ping => 0x9,
            Pong => 0xA,
        }
    }
}
struct ParserInnserState {
    incomming_message_sender: Sender<WebSocketDataPayLoad>,
    outcomming_message_sender: Sender<WebSocketDataPayLoad>,
    buf: BytesMut,
    message: BytesMut,
    current_message_type: WebSocketMessageType,
    machine_state: WebSocketActorState,
}
impl ParserInnserState {
    async fn send_incomming_message(&mut self) {
        let _ = self
            .incomming_message_sender
            .send(WebSocketDataPayLoad::text(
                self.message.split_off(0).freeze(),
            ))
            .await;
        self.machine_state = WebSocketActorState::Head;
    }

    /// return false to indicate that the connection should be closed
    async fn process(&mut self) -> bool {
        loop {
            match self.machine_state {
                WebSocketActorState::Head => {
                    if !self.parse_head() {
                        break;
                    }
                }
                WebSocketActorState::Body {
                    len,
                    mask,
                    msg_finished,
                    readed,
                } => {
                    if !self.parse_body(readed, len, mask, msg_finished) {
                        break;
                    }
                }
                WebSocketActorState::Finished => {
                    use WebSocketMessageType::*;
                    match self.current_message_type {
                        Text => {
                            self.send_incomming_message().await;
                        }
                        Binary => {
                            self.send_incomming_message().await;
                        }
                        Ping => {
                            let _ = self
                                .outcomming_message_sender
                                .send(WebSocketDataPayLoad::text(b"pong"[..].into()))
                                .await;
                        }
                        _ => {}
                    }

                    break;
                }
                WebSocketActorState::ConnectionClose => {
                    let _ = self.outcomming_message_sender
                        .send(WebSocketDataPayLoad::close(b"close"[..].into()))
                        .await;
                    return false;
                }
            }
        }
        true
    }
    fn parse_head(&mut self) -> bool {
        if self.buf.remaining() < 2 {
            return false;
        }

        let p = self.buf.get_u8();

        self.current_message_type = WebSocketMessageType::from(p);
        let msg_finished = p & 0x80 == 0x80;

        if self.current_message_type == WebSocketMessageType::Close {
            self.machine_state = WebSocketActorState::ConnectionClose;
            return true;
        }

        let mut len = (self.buf.get_u8() & 0x7f) as usize;
        if len == 126 {
            if self.buf.remaining() < 2 {
                return false;
            }
            len = self.buf.get_u16() as usize;
        } else if len == 127 {
            if self.buf.remaining() < 8 {
                return false;
            }
            len = self.buf.get_u64() as usize;
        }
        if self.buf.remaining() < 4 {
            return false;
        }
        let mask = [
            self.buf.get_u8(),
            self.buf.get_u8(),
            self.buf.get_u8(),
            self.buf.get_u8(),
        ];
        self.machine_state = WebSocketActorState::Body {
            readed: 0,
            len,
            mask,
            msg_finished,
        };
        true
    }
    fn parse_body(
        &mut self,
        mut readed: usize,
        len: usize,
        mask: [u8; 4],
        msg_finished: bool,
    ) -> bool {
        let remain = len - readed;
        let mut real = vec![];
        let new_msg_len = self.buf.len().min(remain);
        for (i, &d) in self.buf[..new_msg_len].iter().enumerate() {
            real.push(d ^ mask[(i + self.message.len()) % 4]);
        }
        self.message.put(&real[..]);
        readed += new_msg_len;
        let _ = self.buf.split_to(new_msg_len);
        if readed == len {
            if self.current_message_type == WebSocketMessageType::Continuation {
                self.machine_state = WebSocketActorState::Head;
            } else {
                self.machine_state = WebSocketActorState::Finished;
            }
        } else {
            self.machine_state = WebSocketActorState::Body {
                readed,
                len,
                mask,
                msg_finished,
            };
            return false;
        }
        true
    }
    fn put(&mut self, data: Bytes) {
        self.buf.put(data);
    }
    fn new(
        incomming_message_sender: Sender<WebSocketDataPayLoad>,
        outcomming_message_sender: Sender<WebSocketDataPayLoad>,
    ) -> Self {
        Self {
            incomming_message_sender,
            buf: BytesMut::with_capacity(1024),
            message: BytesMut::with_capacity(1024),
            machine_state: WebSocketActorState::Head,
            outcomming_message_sender,
            current_message_type: WebSocketMessageType::Text,
        }
    }
}
pub struct WebSocketIncommingMessageParser {
    incomming_message_stream: RecvStream,
    state: ParserInnserState,
}
impl WebSocketIncommingMessageParser {
    pub fn new(
        incomming_message_stream: RecvStream,
        outcommint_message_sender: Sender<WebSocketDataPayLoad>,
    ) -> (Self, Receiver<WebSocketDataPayLoad>) {
        let (incomming_message_sender, rx) = tokio::sync::mpsc::channel(100);
        (
            Self {
                incomming_message_stream,
                state: ParserInnserState::new(incomming_message_sender, outcommint_message_sender),
            },
            rx,
        )
    }
    pub fn start(mut self) {
        tokio::spawn(async move {
            while let Some(d) = self.incomming_message_stream.data().await {
                let d = d.expect("other side closed");
                let chunk_len = d.len();
                self.state.put(d);
                if !self.state.process().await {
                    break;
                }
                self.incomming_message_stream
                    .flow_control()
                    .release_capacity(chunk_len)
                    .expect("release_capacity error");
            }
        });
    }
}

pub struct Http1WebSocketIncommingMessageParser {
    incomming_message_stream: Box<dyn AsyncRead + Send + Sync + Unpin + 'static>,
    state: ParserInnserState,
}
impl Http1WebSocketIncommingMessageParser {
    pub fn new<R: AsyncRead + Send + Sync + Unpin + 'static>(
        incomming_message_stream: R,
        outcomming_message_sender: Sender<WebSocketDataPayLoad>,
    ) -> (Self, Receiver<WebSocketDataPayLoad>) {
        let (incomming_message_sender, rx) = tokio::sync::mpsc::channel(100);
        (
            Self {
                incomming_message_stream: Box::new(incomming_message_stream),
                state: ParserInnserState::new(incomming_message_sender, outcomming_message_sender),
            },
            rx,
        )
    }
    pub fn start(mut self) {
        tokio::spawn(async move {
            while let Ok(d) = self
                .incomming_message_stream
                .read_buf(&mut self.state.buf)
                .await
            {
                if d == 0 {
                    return;
                }
                if !self.state.process().await {
                    break;
                }
            }
        });
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    use bytes::BytesMut;

    #[tokio::test]
    async fn test_parse_head_basic_text_frame() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(1);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(2);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Create a simple text frame: FIN bit set, text type, 5 bytes, with mask
        // 0x81 = 10000001 (FIN bit + text opcode)
        // 0x85 = 10000101 (masked + payload length 5)
        // Mask: [0x01, 0x02, 0x03, 0x04]
        let mut data = BytesMut::new();
        data.put_u8(0x81); // FIN bit set, text frame
        data.put_u8(0x85); // Masked, payload length 5
        data.put_slice(&[0x01, 0x02, 0x03, 0x04]); // Mask key
        state.buf = data;

        let result = state.parse_head();
        assert!(result);
        assert_eq!(state.current_message_type, WebSocketMessageType::Text);
        assert_eq!(state.machine_state, WebSocketActorState::Body {
            len: 5,
            mask: [0x01, 0x02, 0x03, 0x04],
            msg_finished: true,
            readed: 0,
        });
    }

    #[tokio::test]
    async fn test_parse_head_connection_close() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Create a close frame: 0x88 (FIN bit + close opcode)
        let mut data = BytesMut::new();
        data.put_u8(0x88); // FIN bit set, close frame
        data.put_u8(0x00); // Not masked, payload length 0
        state.buf = data;

        let result = state.parse_head();
        assert!(result);
        assert_eq!(state.current_message_type, WebSocketMessageType::Close);
        assert_eq!(state.machine_state, WebSocketActorState::ConnectionClose);
    }

    #[tokio::test]
    async fn test_parse_head_insufficient_data() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Only 1 byte, need at least 2 for basic header
        let mut data = BytesMut::new();
        data.put_u8(0x81);
        state.buf = data;

        let result = state.parse_head();
        assert!(!result);
        // State should remain unchanged since we don't have enough data
        assert_eq!(state.machine_state, WebSocketActorState::Head);
    }

    #[tokio::test]
    async fn test_parse_head_extended_length_126() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Create frame with 126 length (extended 16-bit)
        let mut data = BytesMut::new();
        data.put_u8(0x81); // FIN bit set, text frame
        data.put_u8(0x7E); // Length 126 indicator
        data.put_u16(0x0100); // Extended payload length: 256 bytes
        data.put_slice(&[0x01, 0x02, 0x03, 0x04]); // Mask key
        state.buf = data;

        let result = state.parse_head();
        assert!(result);
        assert_eq!(state.machine_state, WebSocketActorState::Body {
            len: 256,
            mask: [0x01, 0x02, 0x03, 0x04],
            msg_finished: true,
            readed: 0,
        });
    }

    #[tokio::test]
    async fn test_parse_head_extended_length_127() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Create frame with 127 length (extended 64-bit)
        let mut data = BytesMut::new();
        data.put_u8(0x81); // FIN bit set, text frame
        data.put_u8(0x7F); // Length 127 indicator
        data.put_u64(0x0000000000000100); // Extended payload length: 256 bytes
        data.put_slice(&[0x01, 0x02, 0x03, 0x04]); // Mask key
        state.buf = data;

        let result = state.parse_head();
        assert!(result);
        assert_eq!(state.machine_state, WebSocketActorState::Body {
            len: 256,
            mask: [0x01, 0x02, 0x03, 0x04],
            msg_finished: true,
            readed: 0,
        });
    }

    #[tokio::test]
    async fn test_parse_head_insufficient_mask() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Frame header but no mask (only 2 bytes when we need 6 for mask)
        let mut data = BytesMut::new();
        data.put_u8(0x81); // FIN bit set, text frame
        data.put_u8(0x85); // Masked, payload length 5
        state.buf = data;

        let result = state.parse_head();
        assert!(!result); // Should fail due to insufficient mask data
    }

    #[tokio::test]
    async fn test_parse_body_complete() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Setup: Text message type
        state.current_message_type = WebSocketMessageType::Text;

        // Put masked payload data in buffer (unmasked: "hello" = [104, 101, 108, 108, 111])
        // Mask: [0x01, 0x02, 0x03, 0x04]
        // Masked: [104^0x01, 101^0x02, 108^0x03, 108^0x04, 111^0x01] = [105, 103, 111, 104, 110]
        let mut data = BytesMut::new();
        data.put_slice(&[105, 103, 111, 104, 110]); // Masked "hello"
        state.buf = data;

        let mask = [0x01, 0x02, 0x03, 0x04];
        let result = state.parse_body(0, 5, mask, true); // Complete 5-byte payload

        assert!(result);
        assert_eq!(state.machine_state, WebSocketActorState::Finished);
        // The unmasked message should be "hello"
        assert_eq!(state.message.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn test_parse_body_partial() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Setup: Text message type
        state.current_message_type = WebSocketMessageType::Text;

        // Put partial masked payload data in buffer (only 2 of 5 bytes)
        // Mask: [0x01, 0x02, 0x03, 0x04]
        let mut data = BytesMut::new();
        data.put_slice(&[105, 103]); // First 2 bytes of masked "hello"
        state.buf = data;

        let mask = [0x01, 0x02, 0x03, 0x04];
        let result = state.parse_body(0, 5, mask, true); // 5-byte payload, only 2 bytes available

        assert!(!result); // Should return false since not complete
        assert_eq!(state.machine_state, WebSocketActorState::Body {
            readed: 2,
            len: 5,
            mask,
            msg_finished: true,
        });
        assert_eq!(state.message.as_ref(), b"he"); // First 2 unmasked bytes
    }

    #[tokio::test]
    async fn test_parse_body_with_existing_message() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Setup: Text message type and existing partial message
        state.current_message_type = WebSocketMessageType::Text;
        state.message.put_slice(b"hel");

        // Put remaining masked payload data in buffer
        // Mask: [0x01, 0x02, 0x03, 0x04]
        let mut data = BytesMut::new();
        data.put_slice(&[104]); // Last byte of masked "hello"
        state.buf = data;

        let mask = [0x01, 0x02, 0x03, 0x04];
        let result = state.parse_body(3, 5, mask, true); // 5-byte payload, 3 already read, 1 more available

        assert!(!result); // Should return false since not complete (only 4/5 bytes total)
        assert_eq!(state.machine_state, WebSocketActorState::Body {
            readed: 4,
            len: 5,
            mask,
            msg_finished: true,
        });
        assert_eq!(state.message.as_ref(), b"hell"); // Should have "hel" + "l"
    }

    #[tokio::test]
    async fn test_parse_body_continuation_frame() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Setup: Continuation message type
        state.current_message_type = WebSocketMessageType::Continuation;

        // Put masked payload data in buffer
        let mut data = BytesMut::new();
        data.put_slice(&[105, 103, 111, 104, 110]); // Masked "hello"
        state.buf = data;

        let mask = [0x01, 0x02, 0x03, 0x04];
        let result = state.parse_body(0, 5, mask, true); // Complete 5-byte payload

        assert!(result);
        assert_eq!(state.machine_state, WebSocketActorState::Head); // Should return to Head for continuation
        assert_eq!(state.message.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn test_parse_body_empty_payload() {
        let (incoming_tx, _incoming_rx) = mpsc::channel(10);
        let (outgoing_tx, _outgoing_rx) = mpsc::channel(10);

        let mut state = ParserInnserState::new(incoming_tx, outgoing_tx);

        // Setup: Text message type
        state.current_message_type = WebSocketMessageType::Text;

        // Empty buffer
        state.buf = BytesMut::new();

        let mask = [0x01, 0x02, 0x03, 0x04];
        let result = state.parse_body(0, 0, mask, true); // 0-byte payload

        assert!(result);
        assert_eq!(state.machine_state, WebSocketActorState::Finished);
        assert_eq!(state.message.len(), 0);
    }
}
