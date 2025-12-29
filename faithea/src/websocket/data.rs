use bytes::{BufMut, Bytes, BytesMut};
use h2::SendStream;


pub struct WebSocketDataPayLoad {
    _inner:Bytes
}

impl WebSocketDataPayLoad {
    pub fn new(payload:Bytes) -> Self {
        Self { _inner: payload }
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self._inner
    }
    async fn send_head(&self,body_stream:&mut SendStream<Bytes>) -> Result<(), h2::Error> {
        let b = &self._inner;
        let mut buf = BytesMut::new();
        buf.put_u8(0x81);// fin + text
        if b.len() < 126 {
            buf.put_u8(b.len() as u8);
        }else if b.len() < (u16::MAX - 1) as usize  {
            buf.put_u16(b.len() as u16);
        }else {
            buf.put_u64(b.len() as u64);
        }
        body_stream.reserve_capacity(buf.len());
        body_stream.send_data(buf.freeze(), false)
    }
    async fn send_body(self,body_stream:&mut SendStream<Bytes>) -> Result<(), h2::Error> {
        body_stream.reserve_capacity(self._inner.len());
        body_stream.send_data(self._inner, false)
    }
    pub async fn serialize_to_stream(self,body_stream:&mut SendStream<Bytes>) -> Result<(), h2::Error> {
        self.send_head(body_stream).await?;
        self.send_body(body_stream).await
    }
}
