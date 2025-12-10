use http_server::server::HttpServer;



#[tokio::main(flavor = "current_thread")]
async fn main() {
    let  server = HttpServer::new("127.0.0.1:8899");

    server.start().await
}
