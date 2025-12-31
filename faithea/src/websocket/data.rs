use bytes::{BufMut, Bytes, BytesMut};
use h2::SendStream;
use tokio::io::{self, AsyncWrite, AsyncWriteExt};

use crate::websocket::WebSocketMessageType;

pub struct WebSocketDataPayLoad {
    r#type:WebSocketMessageType,
    _inner: Bytes,
}

impl WebSocketDataPayLoad {
    pub fn text(payload: Bytes) -> Self {
        Self { _inner: payload ,r#type:WebSocketMessageType::Text}
    }
    pub fn ping(payload: Bytes) -> Self{
        Self { _inner: payload ,r#type:WebSocketMessageType::Ping}
    }
    pub fn pong(payload: Bytes) -> Self{
        Self { _inner: payload ,r#type:WebSocketMessageType::Pong}
    }
    pub fn close(payload: Bytes) -> Self{
        Self { _inner: payload ,r#type:WebSocketMessageType::Close}
    }
    pub fn binary(payload: Bytes) -> Self{
        Self { _inner: payload ,r#type:WebSocketMessageType::Binary}
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self._inner
    }
    fn generate_head_frame(&self) -> Bytes {
        let b = &self._inner;
        let mut buf = BytesMut::new();
        let op:u8 = self.r#type.into();
        buf.put_u8(0x80 | op); // fin + text

        if b.len() < 126 {
            buf.put_u8(b.len() as u8);
        } else if b.len() < (u16::MAX - 1) as usize {
            buf.put_u16(b.len() as u16);
        } else {
            buf.put_u64(b.len() as u64);
        }
        buf.freeze()
    }
    pub fn write_to_stream(
        data: Bytes,
        body_stream: &mut SendStream<Bytes>,
    ) -> Result<(), h2::Error> {
        body_stream.reserve_capacity(data.len());
        body_stream.send_data(data, false)
    }
    pub async fn write_to_socket<W: AsyncWrite + Unpin>(
        mut data: Bytes,
        socket: &mut W,
    ) -> Result<(), io::Error> {
        socket.write_all_buf(&mut data).await
    }
    pub async fn serialize_to_stream(
        self,
        body_stream: &mut SendStream<Bytes>,
    ) -> Result<(), h2::Error> {
        let head_frame = self.generate_head_frame();
        Self::write_to_stream(head_frame, body_stream)?;
        Self::write_to_stream(self._inner, body_stream)
    }

    pub async fn serialize_to_socket<W: AsyncWrite + Unpin>(
        self,
        socket: &mut W,
    ) -> Result<(), io::Error> {
        let head_frame = self.generate_head_frame();
        Self::write_to_socket(head_frame, socket).await?;
        Self::write_to_socket(self._inner, socket).await
    }
}
