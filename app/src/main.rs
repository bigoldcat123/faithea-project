// #![allow(unused)]
use chenzhonghai_app::test_handler::test_handlers;
use chenzhonghai_app::ws::ws;
use faithea::{res_modifiers, server::HttpServer};

//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let r = HttpServer::builder()
        .mount("/", test_handlers())
        // .mount("/static", handlers!(file_map))
        .cors()
        .guard("/protected/**", async |req| Ok(req))
        .guard("/**", async |e| {
            log::info!("{:?}", e.uri());
            Ok(e)
        })
        .websocket("/ws/{name}", ws)
        .globale_error_handler(async |e: faithea::error::Error| {
            res_modifiers!(format!("some error~~ {:?}", e))
        })
        .static_map(
            "/static/**",
            "/Users/dadigua/Desktop/graduation/front-end-app",
        )
        .static_map("/**", "/Users/dadigua/Desktop/c/pppp/out")
        .tls(
            "/Users/dadigua/Desktop/graduation/key.pem",
            "/Users/dadigua/Desktop/graduation/cert.pem",
        )
        .h2()
        .host("0.0.0.0")
        .port(443)
        .build()
        .run()
        .await;
    println!("{:?}", r);
}
