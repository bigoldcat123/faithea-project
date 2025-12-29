use bytes::{Buf, BufMut, BytesMut};
use h2::RecvStream;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::websocket::data::WebSocketDataPayLoad;

pub mod data;

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
}
pub struct WebSocketIncommingMessageParser {
    incomming_message_stream: RecvStream,
    incomming_message_sender: Sender<WebSocketDataPayLoad>,
    buf: BytesMut,
    message: BytesMut,
    state: WebSocketActorState,
}
impl WebSocketIncommingMessageParser {
    pub fn new(incomming_message_stream: RecvStream) -> (Self, Receiver<WebSocketDataPayLoad>) {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        (
            Self {
                incomming_message_sender: tx,
                incomming_message_stream,
                state: WebSocketActorState::Head,
                buf: BytesMut::with_capacity(2048),
                message: BytesMut::with_capacity(2048),
            },
            rx,
        )
    }
    fn parse_head(&mut self) -> bool {
        if self.buf.remaining() < 2 {
            return false;
        }

        let p = self.buf.get_u8();
         let msg_finished = p & 0x80 == 0x80;

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
        self.state = WebSocketActorState::Body {
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
            if msg_finished {
                self.state = WebSocketActorState::Finished;
            } else {
                self.state = WebSocketActorState::Head;
            }
        } else {
            self.state = WebSocketActorState::Body {
                readed,
                len,
                mask,
                msg_finished,
            };
            return false;
        }
        true
    }
    pub fn start(mut self) {
        tokio::spawn(async move {
            while let Some(d) = self.incomming_message_stream.data().await {
                let d = d.expect("other side closed");
                let chunk_len = d.len();
                self.buf.put(d);
                loop {
                    match self.state {
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
                            let _ = self
                                .incomming_message_sender
                                .send(WebSocketDataPayLoad::new(
                                    self.message.split_off(0).freeze(),
                                ))
                                .await;
                            self.state = WebSocketActorState::Head;
                            break;
                        }
                    }
                }

                self.incomming_message_stream
                    .flow_control()
                    .release_capacity(chunk_len)
                    .expect("release_capacity error");
            }
        });
    }
}

// pub async fn decode_ws_frame(
//     stream: &mut RecvStream,
//     sender: Sender<WebSocketDataPayLoad>,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let mut buf = BytesMut::with_capacity(2048);
//     let mut len: Option<usize> = None;
//     let mut mask: Option<[u8; 4]> = None;
//     let mut readed = 0;
//     let mut msg = BytesMut::with_capacity(2048);
//     let mut msg_finished = false;
//     while let Some(chunk) = stream.data().await {
//         let chunk = chunk?;
//         let chunk_len = chunk.len();

//         buf.put(chunk);

//         while buf.has_remaining() {
//             if let Some(len_) = len
//                 && let Some(mask_) = mask
//             {
//                 let remain = len_ - readed;
//                 let mut real = vec![];
//                 let new_msg_len = buf.len().min(remain as usize);
//                 for (i, &d) in buf[..new_msg_len].iter().enumerate() {
//                     real.push(d ^ mask_[(i + msg.len()) % 4]);
//                 }
//                 msg.put(&real[..]);
//                 readed += new_msg_len;
//                 let _ = buf.split_to(new_msg_len);
//                 println!("readed {readed}",);

//                 if readed == len_ {
//                     if msg_finished {
//                         // println!("{:?} -> {}", msg, msg.len());
//                         let _ = sender
//                             .send(WebSocketDataPayLoad::new(msg.split_off(0).freeze()))
//                             .await;
//                     }
//                     readed = 0;
//                     len = None;
//                     mask = None;
//                     break;
//                 }
//             } else {
//                 if buf.remaining() < 2 {
//                     break;
//                 }
//                 let p = buf.get_u8();
//                 println!("{:x}", p);
//                 if p & 0x80 == 0x80 {
//                     msg_finished = true;
//                 } else {
//                     msg_finished = false;
//                 }

//                 let mut len_ = (buf.get_u8() & 0x7f) as usize;
//                 if len_ == 126 {
//                     if buf.remaining() < 2 {
//                         break;
//                     }
//                     len_ = buf.get_u16() as usize;
//                 } else if len_ == 127 {
//                     if buf.remaining() < 8 {
//                         break;
//                     }
//                     len_ = buf.get_u64() as usize;
//                 }
//                 len = Some(len_);

//                 println!("len : {len_}",);

//                 if buf.remaining() < 4 {
//                     break;
//                 }
//                 mask = Some([buf.get_u8(), buf.get_u8(), buf.get_u8(), buf.get_u8()]);
//             }
//         }
//         stream.flow_control().release_capacity(chunk_len).unwrap();
//     }
//     Ok(())
// }

// struct H2ResponseActor {
//     rx: Receiver<HttpResponse>,
// }
