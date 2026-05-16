use std::{fmt::Display, str::Utf8Error};

use bytes::{BufMut, Bytes, BytesMut};

use crate::{BytesSink, websocket::WebSocketMessageType};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WebSocketDataPayLoad {
    pub r#type: WebSocketMessageType,
    inner: Bytes,
}

impl WebSocketDataPayLoad {
    pub fn text<D: Display>(payload: D) -> Self {
        Self::_text(payload.to_string().into())
    }

    pub(crate) fn _text(payload: Bytes) -> Self {
        Self {
            inner: payload,
            r#type: WebSocketMessageType::Text,
        }
    }

    pub fn ping(payload: Bytes) -> Self {
        Self {
            inner: payload,
            r#type: WebSocketMessageType::Ping,
        }
    }

    pub fn pong(payload: Bytes) -> Self {
        Self {
            inner: payload,
            r#type: WebSocketMessageType::Pong,
        }
    }

    pub fn close(payload: Bytes) -> Self {
        Self {
            inner: payload,
            r#type: WebSocketMessageType::Close,
        }
    }

    pub fn binary(payload: Bytes) -> Self {
        Self {
            inner: payload,
            r#type: WebSocketMessageType::Binary,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.as_bytes())
    }

    pub fn message_type(&self) -> WebSocketMessageType {
        self.r#type
    }

    fn generate_head_frame(&self) -> Bytes {
        let payload = &self.inner;
        let mut buf = BytesMut::new();
        let op: u8 = self.r#type.into();
        buf.put_u8(0x80 | op);

        if payload.len() < 126 {
            buf.put_u8(payload.len() as u8);
        } else if payload.len() <= u16::MAX as usize {
            buf.put_u8(126);
            buf.put_u16(payload.len() as u16);
        } else {
            buf.put_u8(127);
            buf.put_u64(payload.len() as u64);
        }
        buf.freeze()
    }

    pub fn into_frame_bytes(self) -> Bytes {
        let head = self.generate_head_frame();
        let mut frame = BytesMut::with_capacity(head.len() + self.inner.len());
        frame.put(head);
        frame.put(self.inner);
        frame.freeze()
    }

    pub(crate) async fn write_to_socket<W: BytesSink + ?Sized>(
        data: &[u8],
        socket: &mut W,
    ) -> Result<(), String> {
        socket.write_all2(data).await
    }

    pub async fn serialize_to_socket<W: BytesSink + ?Sized>(
        self,
        socket: &mut W,
    ) -> Result<(), String> {
        if self.r#type.is_control() && self.inner.len() > 125 {
            return Err("control frame payload must be 125 bytes or less".to_string());
        }
        let head_frame = self.generate_head_frame();
        Self::write_to_socket(&head_frame, socket).await?;
        Self::write_to_socket(&self.inner, socket).await?;
        socket.flush2().await
    }
}
