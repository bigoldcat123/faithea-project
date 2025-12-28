use std::{net::SocketAddr, sync::Arc};

use bytes::{Buf, BufMut, BytesMut};
use h2::{RecvStream, server::Builder};
use http::Method;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    sync::mpsc::Sender,
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::HttpRequest,
    response::{HttpResponse, ResponseBody},
    server::{builder::TlsConfig, process_request}, websocket::data::WebSocketDataPayLoad,
};

pub struct H2Server {
    pub(crate) tls: Option<TlsConfig>,
    pub(crate) addr: SocketAddr,
    pub(crate) handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    pub(crate) guards: Arc<GuardTire>,
}

impl H2Server {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "HTTP{} server starting on http{}://{} using http2",
            if self.tls.is_some() { "S" } else { "" },
            if self.tls.is_some() { "s" } else { "" },
            self.addr,
        );
        println!("Press Ctrl+C to stop the server");
        let listener = TcpListener::bind(self.addr).await?;
        match self.tls {
            Some(ref cfg) => {
                let acceptor = cfg.tls_acceptor()?;

                loop {
                    if let Ok((socket, addr)) = listener.accept().await
                        && let Ok(socket) = acceptor.clone().accept(socket).await
                    {
                        let _ = self.deal_with(socket, addr).await;
                    }
                }
            }
            None => loop {
                if let Ok((socket, addr)) = listener.accept().await {
                    let _ = self.deal_with(socket, addr).await;
                }
            },
        }
    }

    async fn deal_with<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
        &self,
        socket: IO,
        _addr: SocketAddr,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("client {} enter", _addr);

        let guards = self.guards.clone();
        let handlers = self.handlers.clone();
        tokio::spawn(async move {
            let e = process(socket, guards, handlers).await;
            println!("{:?}", e);
            println!("client {} left", _addr);
        });

        Ok(())
    }
}

async fn process<IO: AsyncRead + AsyncWrite + Unpin + Send + Sync + 'static>(
    socket: IO,
    guards: Arc<GuardTire>,
    handlers: Arc<HandlerTire>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut h2 = Builder::new()
        .enable_connect_protocol()
        .handshake(socket)
        .await?;
    // let mut h2 = h2::server::handshake(socket).await?;

    while let Some(req) = h2.accept().await {
        let (mut request, respond) = req?;
        let guards = guards.clone();
        let handlers = handlers.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<HttpResponse>(16);

        tokio::spawn(async move {
            let mut respond = respond;
            while let Some(r) = rx.recv().await {
                let _ = r.serialize_to_socket_h2(&mut respond).await;
            }
        });
        tokio::spawn(async move {
            if request.method() == Method::CONNECT {
                // before_open();
                let mut r = HttpResponse::new();
                let (ws_tx, rx) = tokio::sync::mpsc::channel::<WebSocketDataPayLoad>(128);
                r.set_body(ResponseBody::WsBody(rx));
                tx.send(r).await.unwrap();

                tokio::spawn(async move {
                    decode_ws_frame(request.body_mut(), ws_tx).await.unwrap();
                });
            } else {
                let request = HttpRequest::parse_h2(request).await.unwrap();

                process_request(guards, handlers, request, tx).await;
            }
        });
    }

    Ok(())
}
async fn decode_ws_frame(
    stream: &mut RecvStream,
    sender: Sender<WebSocketDataPayLoad>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = BytesMut::with_capacity(2048);
    let mut len: Option<usize> = None;
    let mut mask: Option<[u8; 4]> = None;
    let mut readed = 0;
    let mut msg = BytesMut::with_capacity(2048);
    let mut msg_finished = false;
    while let Some(chunk) = stream.data().await {
        let chunk = chunk?;
        let chunk_len = chunk.len();

        buf.put(chunk);

        while buf.has_remaining() {
            if let Some(len_) = len
                && let Some(mask_) = mask
            {
                let remain = len_ - readed;
                let mut real = vec![];
                let new_msg_len = buf.len().min(remain as usize);
                for (i, &d) in buf[..new_msg_len].iter().enumerate() {
                    real.push(d ^ mask_[(i + msg.len()) % 4]);
                }
                msg.put(&real[..]);
                readed += new_msg_len;
                let _ = buf.split_to(new_msg_len);
                println!("readed {readed}",);

                if readed == len_ {
                    if msg_finished {
                        // println!("{:?} -> {}", msg, msg.len());
                        let _ = sender.send(WebSocketDataPayLoad::new(msg.split_off(0).freeze())).await;
                    }
                    readed = 0;
                    len = None;
                    mask = None;
                    break;
                }
            } else {
                if buf.remaining() < 2 {
                    break;
                }
                let p = buf.get_u8();
                println!("{:x}", p);
                if p & 0x80 == 0x80 {
                    msg_finished = true;
                } else {
                    msg_finished = false;
                }

                let mut len_ = (buf.get_u8() & 0x7f) as usize;
                if len_ == 126 {
                    if buf.remaining() < 2 {
                        break;
                    }
                    len_ = buf.get_u16() as usize;
                } else if len_ == 127 {
                    if buf.remaining() < 8 {
                        break;
                    }
                    len_ = buf.get_u64() as usize;
                }
                len = Some(len_);

                println!("len : {len_}",);

                if buf.remaining() < 4 {
                    break;
                }
                mask = Some([buf.get_u8(), buf.get_u8(), buf.get_u8(), buf.get_u8()]);
            }
        }
        stream.flow_control().release_capacity(chunk_len).unwrap();
    }
    Ok(())
}

// struct H2ResponseActor {
//     rx: Receiver<HttpResponse>,
// }
