use faithea::data::inbound::FromRequest;
// use faithea::handler::types::HttpHandlerError;
use faithea::request::HttpRequest;
use faithea::request::TryFromRequest;
use faithea::request::error::ParseHandlerParamError;
use faithea::{data::Json, post};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Stu {
    name: String,
    age: i32,
}

impl<'a> TryFromRequest<'a> for Stu {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, ParseHandlerParamError> {
        // let body = _req.body().as_mut().unwrap();
        Ok(Stu {
            name: "from req".into(),
            age: 111,
        })
    }
}

#[post("/fromRequest")]
pub async fn from_request(stu: FromRequest<Stu>) {
    Json(stu.into_inner())
}
