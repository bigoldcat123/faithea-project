use std::{
    error::Error,
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use http::{
    HeaderMap,
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    },
};
use rustls::{
    crypto::ring,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio_rustls::TlsAcceptor;

use faithea_websocket::WebSocket;

use crate::{
    ResponseModifier,
    guard::{GuardResultTrait, GuardTire, RawGuardTrait},
    handler::HandlerTire,
    request::HttpRequest,
    response::{HttpResponse, HttpResponseModifier},
    server::{HandlerModifier, Server, http1::H1Server, http2::H2Server},
    util::static_map,
};
pub trait GlobaleHandlerResponseRaw: Future<Output = ResponseModifier> + Send + 'static {}
impl<T: Future<Output = ResponseModifier> + Send + 'static> GlobaleHandlerResponseRaw for T {}
pub trait GlobaleHandlerRaw<R: GlobaleHandlerResponseRaw>:
    Fn(crate::error::Error) -> R + Send + Sync + 'static
{
}
impl<R: GlobaleHandlerResponseRaw, T: Fn(crate::error::Error) -> R + Send + Sync + 'static>
    GlobaleHandlerRaw<R> for T
{
}

pub(crate) type GlobalErrorHandler = Box<
    dyn Fn(crate::error::Error) -> Pin<Box<dyn GlobaleHandlerResponseRaw>> + Send + Sync + 'static,
>;

pub(crate) struct TlsConfig {
    pub(crate) key: PathBuf,
    pub(crate) cert: PathBuf,
    pub(crate) h2: bool,
}

impl TlsConfig {
    pub(crate) fn tls_acceptor(&self) -> Result<TlsAcceptor, Box<dyn Error>> {
        ring::default_provider()
            .install_default()
            .expect("install ring");
        let certs =
            CertificateDer::pem_file_iter(self.cert.as_path())?.collect::<Result<Vec<_>, _>>()?;
        let key = PrivateKeyDer::from_pem_file(self.key.as_path())?;

        let mut config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;
        if self.h2 {
            config.alpn_protocols = vec![b"h2".to_vec(), b"http1.1".to_vec()];
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
    error_handler: Option<GlobalErrorHandler>,
}
impl HttpServerBuilder {
    pub fn globale_error_handler<H, R>(mut self, handler: H) -> Self
    where
        H: GlobaleHandlerRaw<R>,
        R: GlobaleHandlerResponseRaw,
    {
        self.error_handler = Some(Box::new(move |err| Box::pin(handler(err))));
        self
    }
    pub fn static_map(mut self, url_prefix: &str, path_to_dir: &'static str) -> Self {
        // let path_to_dir = path_to_dir.to_string();
        self.handlers
            .get(url_prefix, move |req: HttpRequest| async move {
                let mut a = static_map(&req, path_to_dir).await;
                let mut res = HttpResponse::new();
                a.modify(&mut res).await?;
                Ok(res)
            });
        self
    }
    pub fn guard<F, O, P>(mut self, route: P, f: F) -> Self
    where
        F: RawGuardTrait<O>,
        O: GuardResultTrait,
        P: AsRef<str>,
    {
        self.guards.add(route, f);
        self
    }

    pub fn mount(mut self, pre_fix: &'static str, handlers: Vec<HandlerModifier>) -> Self {
        self.handlers.mount(pre_fix, handlers);
        self
    }

    /// Proxies matching requests to `target`.
    ///
    /// A trailing `/**` appends the unmatched path to the target URL.
    // pub fn proxy(mut self, route: &str, target: &str) -> Self {
    //     self.handlers.proxy(route, target);
    //     self
    // }

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
        self.port(443).host("0.0.0.0")
    }

    pub fn websocket<F, R>(mut self, route: &str, ws_handler: F) -> Self
    where
        F: Fn(WebSocket, HttpRequest) -> R + Send + Sync + 'static + Copy,
        R: Future<Output = ()> + 'static + Send,
    {
        self.handlers.websoekct_h2(route, ws_handler);
        self.handlers.websoekct_h1(route, ws_handler);
        self
    }

    pub fn build(self) -> Server {
        if self.h2 {
            Server::H2Server(H2Server {
                addr: self.addr,
                handlers: Arc::new(self.handlers),
                guards: Arc::new(self.guards),
                tls: self.tls,
                error_handler: self.error_handler.map(Arc::new),
            })
        } else {
            Server::H1Server(H1Server {
                addr: self.addr,
                handlers: Arc::new(self.handlers),
                guards: Arc::new(self.guards),
                tls: self.tls,
                error_handler: self.error_handler.map(Arc::new),
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
            error_handler: None,
        }
    }
}
