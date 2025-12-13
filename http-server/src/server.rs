//! HTTP server implementation using Tokio.
//!
//! This module provides the main HTTP server struct [`HttpServer`] that
//! binds to a TCP socket and processes incoming HTTP connections asynchronously.
//!
//! # Features
//!
//! - **Async I/O**: Built on Tokio for non-blocking network operations
//! - **Concurrent Processing**: Spawns a new task for each connection
//! - **Pipeline Architecture**: Separates request reading from response writing
//! - **Integration**: Works with [`HandlerTire`] for routing and [`GuardTire`] for middleware
//!
//! # Usage
//!
//! ```rust
//! use http_server::{HttpServer, HandlerTire, GuardTire};
//!
//! #[tokio::main]
//! async fn main() {
//!     let handlers = HandlerTire::default();
//!     let guards = GuardTire::default();
//!     let server = HttpServer::new("127.0.0.1:8080", handlers, guards);
//!     server.start().await;
//! }
//! ```

use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use bytes::BytesMut;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, Sender},
};

use crate::{
    guard::GuardTire,
    handler::HandlerTire,
    request::{HttpRequest, parse_http_frame},
    response::HttpResponse,
    route::Route,
};

/// Main HTTP server that listens for incoming connections and processes requests.
///
/// The server binds to a TCP address and spawns a new asynchronous task for
/// each incoming connection. Each connection is processed through a pipeline:
///
/// 1. **Accept connection** from `TcpListener`
/// 2. **Parse HTTP request** from the socket
/// 3. **Execute guard middleware** for request validation
/// 4. **Route to handler** based on URL pattern matching
/// 5. **Send response** back to client
///
/// The server uses a channel-based architecture where request parsing and
/// response writing happen in separate tasks, allowing for pipelining.
///
/// # Fields
///
/// - `addr`: The socket address the server is bound to
/// - `handlers`: Shared reference to the routing trie for request handling
/// - `guards`: Shared reference to the guard trie for request validation
pub struct HttpServer {
    /// Socket address the server is bound to
    addr: SocketAddr,
    /// Shared reference to handler routing trie
    handlers: Arc<HandlerTire>,
    /// Shared reference to guard middleware trie
    guards: Arc<GuardTire>,
}

impl HttpServer {
    /// Creates a new `HttpServer` instance.
    ///
    /// This method resolves the provided address to a [`SocketAddr`] and
    /// prepares the server with the given handler and guard collections.
    /// The server is not started until [`start`](HttpServer::start) is called.
    ///
    /// # Arguments
    ///
    /// * `a` - Address to bind to (any type implementing [`ToSocketAddrs`])
    /// * `handler` - Handler trie for routing requests to handlers
    /// * `guards` - Guard trie for request validation middleware
    ///
    /// # Returns
    ///
    /// A new `HttpServer` instance ready to be started.
    ///
    /// # Panics
    ///
    /// Panics if the address cannot be resolved to a valid socket address.
    /// This is appropriate for server startup where a binding failure should
    /// be fatal.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use http_server::{HttpServer, HandlerTire, GuardTire};
    ///
    /// let handlers = HandlerTire::default();
    /// let guards = GuardTire::default();
    /// let server = HttpServer::new("127.0.0.1:8080", handlers, guards);
    /// ```
    pub fn new<A: ToSocketAddrs>(a: A, handler: HandlerTire, guards: GuardTire) -> Self {
        Self {
            addr: a.to_socket_addrs().unwrap().next().unwrap(),
            handlers: Arc::new(handler),
            guards: Arc::new(guards),
        }
    }

    /// Starts the HTTP server and begins accepting connections.
    ///
    /// This method binds to the configured address and enters an infinite loop
    /// accepting incoming TCP connections. For each connection, it spawns a
    /// new asynchronous task that processes the request through the full
    /// HTTP pipeline.
    ///
    /// The server runs indefinitely until the process is terminated.
    ///
    /// # Returns
    ///
    /// This method does not return unless a fatal error occurs during binding.
    /// In normal operation, it runs forever processing connections.
    ///
    /// # Panics
    ///
    /// Panics if the server cannot bind to the configured address. This is
    /// appropriate for server startup where a binding failure should be fatal.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use http_server::{HttpServer, HandlerTire, GuardTire};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let handlers = HandlerTire::default();
    ///     let guards = GuardTire::default();
    ///     let server = HttpServer::new("127.0.0.1:8080", handlers, guards);
    ///     server.start().await;
    /// }
    /// ```
    pub async fn start(self) {
        let server = TcpListener::bind(self.addr).await.unwrap();
        loop {
            let (socket, addr) = server.accept().await.unwrap();
            println!("new client -> {}", addr);
            let handlers = Arc::clone(&self.handlers);
            let guards = Arc::clone(&self.guards);
            tokio::spawn(async move {
                let e = process(socket, handlers, guards).await;
                println!("{:?}", e);
                println!(" client left -> {}", addr);
            });
        }
    }
}

async fn process(
    socket: TcpStream,
    handlers: Arc<HandlerTire>,
    guards: Arc<GuardTire>,
) -> Result<(), String> {
    let (mut reader, mut writer) = socket.into_split();
    let (tx, mut rx) = mpsc::channel::<HttpResponse>(10);

    // Spawn writer task that consumes responses from the channel and writes them to the socket
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            message.serialize_to_socket(&mut writer).await;
        }
    });

    let mut buf = BytesMut::with_capacity(4096);
    loop {
        let req = parse_http_frame(&mut reader, &mut buf).await?;
        // println!("{:?}", req);

        match guards.guard(&req.req_line.url.clone()[..], req).await {
            Ok(req) => {
                handle_request(handlers.clone(), req, tx.clone()).await;
            }
            Err(res) => {
                let _ = tx.send(res).await;
            }
        }
    }
}

async fn handle_request(
    handlers: Arc<HandlerTire>,
    mut req: HttpRequest,
    tx: Sender<HttpResponse>,
) {
    if let Some((_matched_url, handler)) =
        handlers.get_handler(&req.req_line.url, &req.req_line.method)
    {
        req.process_routes(
            &_matched_url,
            &Route::from(req.req_line.url.as_str()),
        );

        match handler(req).await {
            Ok(res) => {
                let _ = tx.send(res).await;
            }
            Err(_s) => {
                let _ = tx.send(HttpResponse::error(_s)).await;
            }
        }
    } else {
        let _ = tx.send(HttpResponse::not_found()).await;
    }
}
