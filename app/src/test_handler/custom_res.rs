use faithea::{
    data::Json, get, header::{CONTENT_LENGTH, HeaderValue}, res_modifiers, response::{HttpResponseModifier, HttpResponseModifierFuture, cors::CORS}
};
use serde_json::json;

struct MyCustomType {
    name: String,
}

impl HttpResponseModifier for MyCustomType {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut faithea::response::HttpResponse,
    ) -> HttpResponseModifierFuture<'a> {
        Box::pin(async move {
            res.add_header("some-custom-header", self.name.parse().unwrap());
            res.add_header(CONTENT_LENGTH, HeaderValue::from_static("0"));
            Ok(())
        })
    }
}

#[get("/custom_res")]
pub  async fn custom_res() {
    log::info!("{}","hello");
    MyCustomType {
        name: "Hello".into(),
    }
}

#[get("/custom_res2")]
async fn custom_res2() {
    res_modifiers!(
        MyCustomType {
            name: "Hello".into(),
        },
        Json(json!({
            "name":"something~"
        })),
        CORS,
    )
}
