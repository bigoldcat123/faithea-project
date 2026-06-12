use std::{future::Future, pin::Pin};

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Async byte source shared by Faithea protocol parsers.
///
/// Implementors append newly-read bytes to the provided buffer and report how
/// many bytes were appended. Returning `Ok(0)` represents EOF for transports
/// that do not expose a stronger end-of-stream signal.
pub trait BytesSource: Send {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>>;

    fn is_end(&self) -> bool;
}

impl<T: AsyncRead + Send + Unpin> BytesSource for T {
    fn is_end(&self) -> bool {
        false
    }

    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        Box::pin(async move {
            AsyncReadExt::read_buf(self, buf)
                .await
                .map_err(|err| err.to_string())
        })
    }
}

impl<'b> BytesSource for Box<dyn BytesSource + 'b> {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        self.as_mut().read_buf2(buf)
    }

    fn is_end(&self) -> bool {
        self.as_ref().is_end()
    }
}

impl<'b> BytesSource for Box<dyn BytesSource + Send + Sync + Unpin + 'b> {
    fn read_buf2<'a>(
        &'a mut self,
        buf: &'a mut BytesMut,
    ) -> Pin<Box<dyn Future<Output = Result<usize, String>> + Send + 'a>> {
        self.as_mut().read_buf2(buf)
    }

    fn is_end(&self) -> bool {
        self.as_ref().is_end()
    }
}

/// Async byte sink used to serialize protocol frames to a transport.
pub trait BytesSink: Send {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>>;
}

impl<T: AsyncWrite + Send + Unpin> BytesSink for T {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        Box::pin(async move { self.write_all(buf).await.map_err(|err| err.to_string()) })
    }

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move { self.flush().await.map_err(|err| err.to_string()) })
    }
}

impl<'b> BytesSink for Box<dyn BytesSink + 'b> {
    fn write_all2<'a>(
        &'a mut self,
        buf: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
        self.as_mut().write_all2(buf)
    }

    fn flush2(&mut self) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + '_>> {
        self.as_mut().flush2()
    }
}
