use faithea::{get, res_modifiers, response::cors::CORS};


#[get("/")]
async fn hello_world() {
    res_modifiers!("Hello,World", CORS)
}
