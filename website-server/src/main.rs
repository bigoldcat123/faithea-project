#[tokio::main]
async fn main() {
    env_logger::init();
    faithea::server::HttpServer::builder()
        .static_map(
            "/**",
            "./faithea-website/out",
        )
        .port(8080)
        .build()
        .run()
        .await
        .unwrap();
}
