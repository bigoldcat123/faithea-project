use std::str::Bytes;

use h2::server;
use http::{Response, StatusCode};
use tokio::net::TcpListener;

#[tokio::main(flavor="current_thread")]
pub async fn main() {
    let listener = TcpListener::bind("127.0.0.1:5928").await.unwrap();

    // Accept all incoming TCP connections.
    loop {
        if let Ok((socket, _peer_addr)) = listener.accept().await {
            // Spawn a new task to process each connection.
            tokio::spawn(async {
                // Start the HTTP/2 connection handshake
                let mut h2 = server::handshake(socket).await.unwrap();
                // Accept all inbound HTTP/2 streams sent over the
                // connection.
                while let Some(request) = h2.accept().await {
                    let (mut request, mut respond) = request.unwrap();
                    while let Some(Ok(e)) = request.body_mut().data().await {
                        println!("Received request: {:?}", e);
                    }
                    // Build a response with no body
                    let response = Response::builder()
                        .header("content-length", "4")
                        .status(StatusCode::OK)
                        .body(())
                        .unwrap();

                    // Send the response back to the client
                    let mut x = respond.send_response(response ,false)
                        .unwrap();
                    x.send_data("helo".into(), true).unwrap();
                }

            });
        }
    }
}
