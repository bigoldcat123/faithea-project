use faithea::{handlers, server::HttpServer};
use sql_int::{handlers, init_db};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    init_db().await;
    env_logger::init();
    let _ = HttpServer::builder()
        .mount("/", handlers!(handlers::get_user))
        .build().run().await;
}
