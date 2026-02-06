use faithea::{get, response::redirect::Redirect};

#[get("/redirect")]
async fn redirect() {
    println!("redirect");
    Redirect("https://localhost:443/")
}
