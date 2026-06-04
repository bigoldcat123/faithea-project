use faithea::{get, response::redirect::Redirect};

#[get("/redirect")]
async fn redirect() {
    println!("redirect");
    Redirect("https://www.baidu.com")
}
