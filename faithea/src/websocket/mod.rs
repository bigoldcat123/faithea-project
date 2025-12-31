use bytes::{Buf, BufMut, Bytes, BytesMut};
use h2::RecvStream;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::mpsc::{Receiver, Sender},
};

use crate::websocket::data::WebSocketDataPayLoad;

pub mod data;
pub mod socket;

#[derive(Debug)]
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
                    println!("{}", "other side close!");
                    return;
                }
                if !self.state.process().await {
                    break;
                }
            }
        });
    }
}
