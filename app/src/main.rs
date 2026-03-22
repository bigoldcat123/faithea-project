// #![allow(unused)]
use chenzhonghai_app::static_file_map::file_map;
use chenzhonghai_app::test_handler::test_handlers;
use chenzhonghai_app::{ws::ws};
use faithea::{handlers, res_modifiers, server::HttpServer};

//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let r = HttpServer::builder()
        .mount("/", test_handlers())
        .mount("/static", handlers!(file_map))
        .cors()
        .guard("/protected/**", async |req| Ok(req))
        .guard("/**", async |e| {
            // println!("{e:?}",);
            Ok(e)
        })
        .websocket("/ws/{name}", ws)
        .globale_error_handler(async |e: faithea::error::Error| {
            res_modifiers!(format!("some error~~ {:?}", e))
        })
        // .tls(
        //     "/Users/dadigua/Desktop/graduation/key.pem",
        //     "/Users/dadigua/Desktop/graduation/cert.pem",
        // )
        // .h2()
        // .host("0.0.0.0")
        // .port(443)
        .build()
        .run()
        .await;
    println!("{:?}", r);
}
