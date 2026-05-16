use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{BytesSource, websocket::data::WebSocketDataPayLoad};

pub mod data;
pub mod socket;

const FIN_BIT_MASK: u8 = 0x80;
const RSV_MASK: u8 = 0x70;
const OPCODE_MASK: u8 = 0x0F;
const MASK_BIT_MASK: u8 = 0x80;
const PAYLOAD_LEN_MASK: u8 = 0x7F;
const EXTENDED_LEN_16: usize = 126;
const EXTENDED_LEN_64: usize = 127;
const MASK_KEY_LEN: usize = 4;
const DEFAULT_MAX_FRAME_SIZE: usize = 16 << 20;
const DEFAULT_MAX_MESSAGE_SIZE: usize = 64 << 20;

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum WebSocketMessageType {
    Continuation,
    Text,
    Binary,
    Close,
    Ping,
    Pong,
}

impl WebSocketMessageType {
    fn from_opcode(opcode: u8) -> Result<Self, ProtocolError> {
        match opcode {
            0x0 => Ok(WebSocketMessageType::Continuation),
            0x1 => Ok(WebSocketMessageType::Text),
            0x2 => Ok(WebSocketMessageType::Binary),
            0x8 => Ok(WebSocketMessageType::Close),
            0x9 => Ok(WebSocketMessageType::Ping),
            0xA => Ok(WebSocketMessageType::Pong),
            value => Err(ProtocolError::InvalidOpcode(value)),
        }
    }

    pub(crate) fn is_control(self) -> bool {
        matches!(
            self,
            WebSocketMessageType::Close | WebSocketMessageType::Ping | WebSocketMessageType::Pong
        )
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidOpcode(u8),
    NonZeroReservedBits,
    UnmaskedFrameFromClient,
    FragmentedControlFrame,
    ControlFrameTooBig,
    FrameTooBig { size: usize, max_size: usize },
    MessageTooBig { size: usize, max_size: usize },
    UnexpectedContinuation,
    ExpectedContinuation(WebSocketMessageType),
    InvalidUtf8,
    Io(String),
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ProtocolError {}

#[derive(Debug, Clone, Eq, PartialEq)]
struct FrameHeader {
    fin: bool,
    opcode: WebSocketMessageType,
    mask: [u8; 4],
    len: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct Frame {
    fin: bool,
    opcode: WebSocketMessageType,
    payload: Bytes,
}

#[derive(Debug)]
struct FrameCodec {
    buf: BytesMut,
    max_frame_size: usize,
}

impl FrameCodec {
    fn new() -> Self {
        Self {
            buf: BytesMut::with_capacity(8 * 1024),
            max_frame_size: DEFAULT_MAX_FRAME_SIZE,
        }
    }

    fn buffer_mut(&mut self) -> &mut BytesMut {
        &mut self.buf
    }

    fn next_frame(&mut self) -> Result<Option<Frame>, ProtocolError> {
        let Some((header, header_len)) = self.parse_header()? else {
            return Ok(None);
        };

        let frame_len = header_len + header.len;
        if self.buf.len() < frame_len {
            return Ok(None);
        }

        self.buf.advance(header_len);
        let mut payload = self.buf.split_to(header.len);
        apply_mask(&mut payload, header.mask);

        Ok(Some(Frame {
            fin: header.fin,
            opcode: header.opcode,
            payload: payload.freeze(),
        }))
    }

    fn parse_header(&self) -> Result<Option<(FrameHeader, usize)>, ProtocolError> {
        if self.buf.len() < 2 {
            return Ok(None);
        }

        let first = self.buf[0];
        let second = self.buf[1];
        if first & RSV_MASK != 0 {
            return Err(ProtocolError::NonZeroReservedBits);
        }

        let opcode = WebSocketMessageType::from_opcode(first & OPCODE_MASK)?;
        let fin = first & FIN_BIT_MASK != 0;
        let masked = second & MASK_BIT_MASK != 0;
        if !masked {
            return Err(ProtocolError::UnmaskedFrameFromClient);
        }
        if opcode.is_control() && !fin {
            return Err(ProtocolError::FragmentedControlFrame);
        }

        let mut header_len = 2;
        let mut len = (second & PAYLOAD_LEN_MASK) as usize;
        if len == EXTENDED_LEN_16 {
            if self.buf.len() < header_len + 2 {
                return Ok(None);
            }
            len = u16::from_be_bytes([self.buf[2], self.buf[3]]) as usize;
            header_len += 2;
        } else if len == EXTENDED_LEN_64 {
            if self.buf.len() < header_len + 8 {
                return Ok(None);
            }
            let raw_len = u64::from_be_bytes([
                self.buf[2],
                self.buf[3],
                self.buf[4],
                self.buf[5],
                self.buf[6],
                self.buf[7],
                self.buf[8],
                self.buf[9],
            ]);
            len = usize::try_from(raw_len).map_err(|_| ProtocolError::FrameTooBig {
                size: usize::MAX,
                max_size: self.max_frame_size,
            })?;
            header_len += 8;
        }

        if opcode.is_control() && len > 125 {
            return Err(ProtocolError::ControlFrameTooBig);
        }
        if len > self.max_frame_size {
            return Err(ProtocolError::FrameTooBig {
                size: len,
                max_size: self.max_frame_size,
            });
        }
        if self.buf.len() < header_len + MASK_KEY_LEN {
            return Ok(None);
        }

        let mask = [
            self.buf[header_len],
            self.buf[header_len + 1],
            self.buf[header_len + 2],
            self.buf[header_len + 3],
        ];
        header_len += MASK_KEY_LEN;

        Ok(Some((
            FrameHeader {
                fin,
                opcode,
                mask,
                len,
            },
            header_len,
        )))
    }
}

#[derive(Debug)]
struct MessageAssembler {
    current_type: Option<WebSocketMessageType>,
    current_payload: BytesMut,
    max_message_size: usize,
}

impl MessageAssembler {
    fn new() -> Self {
        Self {
            current_type: None,
            current_payload: BytesMut::new(),
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
        }
    }

    fn push_frame(&mut self, frame: Frame) -> Result<Option<WebSocketDataPayLoad>, ProtocolError> {
        match frame.opcode {
            WebSocketMessageType::Text | WebSocketMessageType::Binary => {
                if let Some(message_type) = self.current_type {
                    return Err(ProtocolError::ExpectedContinuation(message_type));
                }

                if frame.fin {
                    self.check_message_size(frame.payload.len())?;
                    if frame.opcode == WebSocketMessageType::Text {
                        std::str::from_utf8(&frame.payload)
                            .map_err(|_| ProtocolError::InvalidUtf8)?;
                        Ok(Some(WebSocketDataPayLoad::_text(frame.payload)))
                    } else {
                        Ok(Some(WebSocketDataPayLoad::binary(frame.payload)))
                    }
                } else {
                    self.current_type = Some(frame.opcode);
                    self.extend_current(&frame.payload)?;
                    Ok(None)
                }
            }
            WebSocketMessageType::Continuation => {
                let Some(message_type) = self.current_type else {
                    return Err(ProtocolError::UnexpectedContinuation);
                };

                self.extend_current(&frame.payload)?;
                if !frame.fin {
                    return Ok(None);
                }

                let payload = self.current_payload.split().freeze();
                self.current_type = None;
                if message_type == WebSocketMessageType::Text {
                    std::str::from_utf8(&payload).map_err(|_| ProtocolError::InvalidUtf8)?;
                    Ok(Some(WebSocketDataPayLoad::_text(payload)))
                } else {
                    Ok(Some(WebSocketDataPayLoad::binary(payload)))
                }
            }
            WebSocketMessageType::Ping => Ok(Some(WebSocketDataPayLoad::ping(frame.payload))),
            WebSocketMessageType::Pong => Ok(Some(WebSocketDataPayLoad::pong(frame.payload))),
            WebSocketMessageType::Close => Ok(Some(WebSocketDataPayLoad::close(frame.payload))),
        }
    }

    fn extend_current(&mut self, payload: &[u8]) -> Result<(), ProtocolError> {
        let new_len = self.current_payload.len() + payload.len();
        self.check_message_size(new_len)?;
        self.current_payload.put_slice(payload);
        Ok(())
    }

    fn check_message_size(&self, size: usize) -> Result<(), ProtocolError> {
        if size > self.max_message_size {
            return Err(ProtocolError::MessageTooBig {
                size,
                max_size: self.max_message_size,
            });
        }
        Ok(())
    }
}

pub struct WebSocketIncommingMessageParser<SOURCE> {
    incomming_message_stream_source: SOURCE,
    incomming_message_sender: Sender<WebSocketDataPayLoad>,
    outcomming_message_sender: Sender<WebSocketDataPayLoad>,
}

impl<SOURCE> WebSocketIncommingMessageParser<SOURCE>
where
    SOURCE: BytesSource + 'static,
{
    pub fn new(
        incomming_message_stream_source: SOURCE,
        outcomming_message_sender: Sender<WebSocketDataPayLoad>,
    ) -> (Self, Receiver<WebSocketDataPayLoad>) {
        let (incoming_sender, incoming_receiver) = tokio::sync::mpsc::channel(100);

        (
            Self {
                incomming_message_stream_source,
                incomming_message_sender: incoming_sender,
                outcomming_message_sender,
            },
            incoming_receiver,
        )
    }

    pub fn start(self) {
        tokio::spawn(read_loop(
            self.incomming_message_stream_source,
            self.incomming_message_sender,
            self.outcomming_message_sender,
        ));
    }
}

pub type WebSocketIncomingMessageParser<SOURCE> = WebSocketIncommingMessageParser<SOURCE>;

async fn read_loop<SOURCE>(
    mut source: SOURCE,
    incoming_sender: Sender<WebSocketDataPayLoad>,
    control_sender: Sender<WebSocketDataPayLoad>,
) where
    SOURCE: BytesSource,
{
    let mut codec = FrameCodec::new();
    let mut assembler = MessageAssembler::new();

    loop {
        match drain_frames(
            &mut codec,
            &mut assembler,
            &incoming_sender,
            &control_sender,
        )
        .await
        {
            Ok(DriverAction::Continue) => {}
            Ok(DriverAction::Close) => break,
            Err(_) => {
                let _ = control_sender
                    .send(WebSocketDataPayLoad::close(Bytes::new()))
                    .await;
                break;
            }
        }

        if source.is_end() {
            break;
        }

        match source.read_buf2(codec.buffer_mut()).await {
            Ok(0) => break,
            Ok(len) => {
                println!("{}",len);
            }
            Err(_) => {
                let _ = control_sender
                    .send(WebSocketDataPayLoad::close(Bytes::new()))
                    .await;
                break;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DriverAction {
    Continue,
    Close,
}

async fn drain_frames(
    codec: &mut FrameCodec,
    assembler: &mut MessageAssembler,
    incoming_sender: &Sender<WebSocketDataPayLoad>,
    control_sender: &Sender<WebSocketDataPayLoad>,
) -> Result<DriverAction, ProtocolError> {
    while let Some(frame) = codec.next_frame()? {
        match assembler.push_frame(frame)? {
            Some(message) if message.r#type == WebSocketMessageType::Ping => {
                control_sender
                    .send(WebSocketDataPayLoad::pong(Bytes::copy_from_slice(
                        message.as_bytes(),
                    )))
                    .await
                    .map_err(|err| ProtocolError::Io(err.to_string()))?;
                incoming_sender
                    .send(message)
                    .await
                    .map_err(|err| ProtocolError::Io(err.to_string()))?;
            }
            Some(message) if message.r#type == WebSocketMessageType::Close => {
                control_sender
                    .send(WebSocketDataPayLoad::close(Bytes::copy_from_slice(
                        message.as_bytes(),
                    )))
                    .await
                    .map_err(|err| ProtocolError::Io(err.to_string()))?;
                incoming_sender
                    .send(message)
                    .await
                    .map_err(|err| ProtocolError::Io(err.to_string()))?;
                return Ok(DriverAction::Close);
            }
            Some(message) => {
                incoming_sender
                    .send(message)
                    .await
                    .map_err(|err| ProtocolError::Io(err.to_string()))?;
            }
            None => {}
        }
    }
    Ok(DriverAction::Continue)
}

fn apply_mask(payload: &mut [u8], mask: [u8; 4]) {
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % MASK_KEY_LEN];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BytesSink;
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
    };
    use tokio::time::{Duration, timeout};

    #[derive(Debug)]
    struct MemorySource {
        chunks: Vec<Bytes>,
        ended: bool,
    }

    impl MemorySource {
        fn new(chunks: Vec<Bytes>) -> Self {
            Self {
                chunks: chunks.into_iter().rev().collect(),
                ended: false,
            }
        }
    }

    impl BytesSource for MemorySource {
        fn read_buf2<'a>(
            &'a mut self,
            buf: &'a mut BytesMut,
        ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
            Box::pin(async move {
                match self.chunks.pop() {
                    Some(chunk) => {
                        let len = chunk.len();
                        buf.put_slice(&chunk);
                        Ok(len)
                    }
                    None => {
                        self.ended = true;
                        Ok(0)
                    }
                }
            })
        }

        fn is_end(&self) -> bool {
            self.ended
        }
    }

    #[derive(Debug, Clone)]
    struct MemorySink {
        written: Arc<Mutex<Vec<u8>>>,
    }

    impl MemorySink {
        fn new() -> Self {
            Self {
                written: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn bytes(&self) -> Vec<u8> {
            self.written.lock().unwrap().clone()
        }
    }

    impl BytesSink for MemorySink {
        fn write_all2<'a>(
            &'a mut self,
            buf: &'a [u8],
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                self.written.lock().unwrap().extend_from_slice(buf);
                Ok(())
            })
        }

        fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
            Box::pin(async move { Ok(()) })
        }
    }

    fn masked_frame(opcode: WebSocketMessageType, fin: bool, payload: &[u8]) -> Bytes {
        let mask = [1, 2, 3, 4];
        let mut buf = BytesMut::new();
        let first = u8::from(opcode) | if fin { FIN_BIT_MASK } else { 0 };
        buf.put_u8(first);
        if payload.len() < EXTENDED_LEN_16 {
            buf.put_u8(MASK_BIT_MASK | payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            buf.put_u8(MASK_BIT_MASK | EXTENDED_LEN_16 as u8);
            buf.put_u16(payload.len() as u16);
        } else {
            buf.put_u8(MASK_BIT_MASK | EXTENDED_LEN_64 as u8);
            buf.put_u64(payload.len() as u64);
        }
        buf.put_slice(&mask);
        for (index, byte) in payload.iter().enumerate() {
            buf.put_u8(byte ^ mask[index % MASK_KEY_LEN]);
        }
        buf.freeze()
    }

    #[test]
    fn frame_header_short_length_waits_for_payload() {
        let mut codec = FrameCodec::new();
        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Text, true, b"hi")[..5]);

        assert_eq!(codec.next_frame().unwrap(), None);
        assert_eq!(codec.buffer_mut().len(), 5);

        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Text, true, b"hi")[5..]);
        let frame = codec.next_frame().unwrap().unwrap();
        assert_eq!(frame.opcode, WebSocketMessageType::Text);
        assert_eq!(frame.payload, &b"hi"[..]);
    }

    #[test]
    fn frame_header_extended_126() {
        let payload = vec![b'a'; 256];
        let mut codec = FrameCodec::new();
        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Binary, true, &payload));

        let frame = codec.next_frame().unwrap().unwrap();
        assert_eq!(frame.opcode, WebSocketMessageType::Binary);
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn frame_header_extended_127() {
        let payload = vec![b'b'; 66_000];
        let mut codec = FrameCodec::new();
        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Binary, true, &payload));

        let frame = codec.next_frame().unwrap().unwrap();
        assert_eq!(frame.payload.len(), payload.len());
        assert_eq!(frame.payload, payload);
    }

    #[test]
    fn unmasked_client_frame_is_protocol_error() {
        let mut codec = FrameCodec::new();
        codec.buffer_mut().put_slice(&[0x81, 0x02, b'h', b'i']);

        assert_eq!(
            codec.next_frame().unwrap_err(),
            ProtocolError::UnmaskedFrameFromClient
        );
    }

    #[test]
    fn rsv_bits_are_protocol_error() {
        let mut codec = FrameCodec::new();
        codec.buffer_mut().put_slice(&[0xC1, 0x80, 1, 2, 3, 4]);

        assert_eq!(
            codec.next_frame().unwrap_err(),
            ProtocolError::NonZeroReservedBits
        );
    }

    #[test]
    fn reserved_opcode_is_protocol_error() {
        let mut codec = FrameCodec::new();
        codec.buffer_mut().put_slice(&[0x83, 0x80, 1, 2, 3, 4]);

        assert_eq!(
            codec.next_frame().unwrap_err(),
            ProtocolError::InvalidOpcode(3)
        );
    }

    #[test]
    fn control_frame_cannot_be_fragmented() {
        let mut codec = FrameCodec::new();
        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Ping, false, b"hi"));

        assert_eq!(
            codec.next_frame().unwrap_err(),
            ProtocolError::FragmentedControlFrame
        );
    }

    #[test]
    fn control_frame_cannot_exceed_125_bytes() {
        let payload = vec![0; 126];
        let mut codec = FrameCodec::new();
        codec
            .buffer_mut()
            .put_slice(&masked_frame(WebSocketMessageType::Ping, true, &payload));

        assert_eq!(
            codec.next_frame().unwrap_err(),
            ProtocolError::ControlFrameTooBig
        );
    }

    #[test]
    fn fragmented_text_is_reassembled() {
        let mut assembler = MessageAssembler::new();
        assert_eq!(
            assembler
                .push_frame(Frame {
                    fin: false,
                    opcode: WebSocketMessageType::Text,
                    payload: Bytes::from_static(b"hel"),
                })
                .unwrap(),
            None
        );
        let message = assembler
            .push_frame(Frame {
                fin: true,
                opcode: WebSocketMessageType::Continuation,
                payload: Bytes::from_static(b"lo"),
            })
            .unwrap()
            .unwrap();

        assert_eq!(message.r#type, WebSocketMessageType::Text);
        assert_eq!(message.as_str().unwrap(), "hello");
    }

    #[test]
    fn unexpected_continuation_is_protocol_error() {
        let mut assembler = MessageAssembler::new();
        assert_eq!(
            assembler
                .push_frame(Frame {
                    fin: true,
                    opcode: WebSocketMessageType::Continuation,
                    payload: Bytes::new(),
                })
                .unwrap_err(),
            ProtocolError::UnexpectedContinuation
        );
    }

    #[tokio::test]
    async fn parser_reads_messages_and_payload_can_serialize_to_sink() {
        let source: Box<dyn BytesSource> = Box::new(MemorySource::new(vec![masked_frame(
            WebSocketMessageType::Text,
            true,
            b"hi",
        )]));
        let sink = MemorySink::new();
        let sink_view = sink.clone();
        let mut sink_writer = sink.clone();
        let (outgoing, _outgoing_rx) = tokio::sync::mpsc::channel(10);
        let (parser, mut incoming) = WebSocketIncommingMessageParser::new(source, outgoing);
        parser.start();

        let msg = timeout(Duration::from_secs(1), incoming.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(msg.as_str().unwrap(), "hi");

        let sink_writer_dyn: &mut dyn BytesSink = &mut sink_writer;
        WebSocketDataPayLoad::binary(Bytes::from_static(&[1, 2, 3]))
            .serialize_to_socket(sink_writer_dyn)
            .await
            .unwrap();

        assert_eq!(sink_view.bytes(), vec![0x82, 0x03, 1, 2, 3]);
    }

    #[tokio::test]
    async fn parser_sends_auto_pongs_and_closes_to_existing_outgoing_channel() {
        let source = MemorySource::new(vec![
            masked_frame(WebSocketMessageType::Ping, true, b"ok"),
            masked_frame(WebSocketMessageType::Close, true, b""),
        ]);
        let (outgoing, mut outgoing_rx) = tokio::sync::mpsc::channel(10);
        let (parser, mut incoming) = WebSocketIncommingMessageParser::new(source, outgoing);
        parser.start();

        let ping = timeout(Duration::from_secs(1), incoming.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ping.r#type, WebSocketMessageType::Ping);
        let close = timeout(Duration::from_secs(1), incoming.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(close.r#type, WebSocketMessageType::Close);

        let auto_pong = timeout(Duration::from_secs(1), outgoing_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(auto_pong.r#type, WebSocketMessageType::Pong);
        assert_eq!(auto_pong.as_bytes(), b"ok");

        let auto_close = timeout(Duration::from_secs(1), outgoing_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(auto_close.r#type, WebSocketMessageType::Close);
    }
}
