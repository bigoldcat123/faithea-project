use faithea::get;

#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
