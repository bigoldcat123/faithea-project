use faithea::{get, request::static_map};

#[get("/**")]
pub async fn file_map() {
    static_map(_req, "/Users/dadigua/Desktop/graduation/front-end-app").await
}
