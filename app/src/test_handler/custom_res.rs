use faithea::{
    data::Json,
    get, res_modifiers,
    response::{HttpResponseModifier, cors::CORS},
};
use serde_json::json;

struct MyCustomType {
    name: String,
}

impl HttpResponseModifier for MyCustomType {
    fn modify<'a>(
        &'a mut self,
        res: &'a mut faithea::response::HttpResponse,
    ) -> std::pin::Pin<
        Box<
            dyn Future<Output = Result<(), faithea::handler::types::HttpHandlerError>>
                + 'a
                + Send
                + Sync,
        >,
    > {
        Box::pin(async move {
            res.add_header("some-custom-header", self.name.parse().unwrap());
            Ok(())
        })
    }
}

#[get("/custom_res")]
async fn custom_res() {
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
