use std::{
    error::Error,
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::Arc,
};

use http::{
    HeaderMap,
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    },
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use tokio_rustls::TlsAcceptor;

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier},
    server::{HandlerModifier, Server, http1::H1Server, http2::H2Server}, websocket::socket::WebSocket,
};

pub(crate) struct TlsConfig {
    pub(crate) key: PathBuf,
    pub(crate) cert: PathBuf,
    pub(crate) h2: bool,
}

impl TlsConfig {
    pub(crate) fn tls_acceptor(&self) -> Result<TlsAcceptor, Box<dyn Error>> {
        let certs =
            CertificateDer::pem_file_iter(self.cert.as_path())?.collect::<Result<Vec<_>, _>>()?;
        let key = PrivateKeyDer::from_pem_file(self.key.as_path())?;

        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        if self.h2 {
            config.alpn_protocols = vec![b"h2".to_vec(),b"http1.1".to_vec()];
        }
        let acceptor = TlsAcceptor::from(Arc::new(config));
        Ok(acceptor)
    }
}

pub struct HttpServerBuilder {
    handlers: HandlerTire,
    guards: GuardTire,
    addr: SocketAddr,
    tls: Option<TlsConfig>,
    h2: bool,
}
impl HttpServerBuilder {
    pub fn guard<F, O, P>(mut self, route: P, f: F) -> Self
    where
        F: Fn(HttpRequest) -> O + 'static + Send + Sync,
        O: Future<Output = Result<HttpRequest, HttpResponse>> + 'static + Send + Sync,
        P: AsRef<str>,
    {
        self.guards.add(route, f);
        self
    }

    pub fn mount(mut self, pre_fix: &'static str, handlers: Vec<HandlerModifier>) -> Self {
        self.handlers.mount(pre_fix, handlers);
        self
    }

    pub fn port(mut self, p: u16) -> Self {
        self.addr.set_port(p);
        self
    }
    pub fn host(mut self, host: &str) -> Self {
        self.addr
            .set_ip(host.parse().expect("in correct ip host eg. 0.0.0.0"));
        self
    }
    pub fn cors(mut self) -> Self {
        self.handlers.options("/**", |_: HttpRequest| async move {
            let mut res = HttpResponse::new();
            let mut header = HeaderMap::new();
            header.insert(ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
            header.insert(ACCESS_CONTROL_ALLOW_HEADERS, "*".parse().unwrap());
            header.insert(
                ACCESS_CONTROL_ALLOW_METHODS,
                "GET, POST, PUT, DELETE".parse().unwrap(),
            );
            header.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());
            header.modify(&mut res).await?;
            Ok(res)
        });
        self
    }
    pub fn h2(mut self) -> Self {
        self.h2 = true;
        if let Some(tls) = self.tls.as_mut() {
            tls.h2 = true;
        }
        self
    }
    pub fn tls<K: AsRef<Path>, C: AsRef<Path>>(mut self, key: K, cert: C) -> Self {
        self.tls = Some(TlsConfig {
            key: key.as_ref().to_path_buf(),
            cert: cert.as_ref().to_path_buf(),
            h2: self.h2,
        });
        self
    }

    pub fn websocket<F,R>(mut self,route:&str,ws_handler:F) -> Self
   where
       F:Fn(WebSocket,HttpRequest) -> R + Send + Sync + 'static,
       R:Future<Output = ()> + 'static + Send
   {
       self.handlers.websoekct_h2(route, ws_handler);
       self
    }

    pub fn build(self) -> Server {
        if self.h2 {
            Server::H2Server(H2Server {
                addr: self.addr,
                handlers: Arc::new(self.handlers),
                guards: Arc::new(self.guards),
                tls: self.tls,
            })
        } else {
            Server::H1Server(H1Server {
                addr: self.addr,
                handlers: Arc::new(self.handlers),
                guards: Arc::new(self.guards),
                tls: self.tls,
            })
        }
    }
}
impl Default for HttpServerBuilder {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            guards: Default::default(),
            addr: "127.0.0.1:8899".to_socket_addrs().unwrap().next().unwrap(),
            tls: None,
            h2: false,
        }
    }
}
