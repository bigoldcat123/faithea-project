use h2::server;
use http::{Response, StatusCode, header::CONTENT_LENGTH};
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8899").await.unwrap();

    // Accept all incoming TCP connections.
    loop {
        if let Ok((socket, _peer_addr)) = listener.accept().await {
            // Spawn a new task to process each connection.
            tokio::spawn(async {
                // Start the HTTP/2 connection handshake
                let mut h2 = server::handshake(socket).await.unwrap();
                // Accept all inbound HTTP/2 streams sent over the
                // connection.
                while let Some(Ok(request)) = h2.accept().await {
                    let (mut request, mut respond) = request;
                    while let Some(Ok(e)) = request.body_mut().data().await {
                        println!("Received request: {:?}", e);
                        println!("{}",request.body().is_end_stream());

                        if request.body().is_end_stream(){
                            break;
                        }

                    }
                    println!("哈哈哈");

                    let (tx, mut rx) = tokio::sync::mpsc::channel::<Response<String>>(64);
                    tokio::spawn(async move {
                        while let Some(r) = rx.recv().await {
                            let (p, b) = r.into_parts();
                            // Send the response back to the client
                            let mut x = respond
                                .send_response(Response::from_parts(p, ()), false)
                                .unwrap();
                            let a = bytes::Bytes::from_owner(b);
                            x.send_data(a, true).unwrap();
                        }
                    });
                    // Build a response with no body
                    let response = Response::builder()
                        .header("content-length", "4")
                        .status(StatusCode::OK)
                        .body("abcd".to_string())
                        .unwrap();
                    let _ = tx.send(response).await;
                }
            });
        }
    }
}
