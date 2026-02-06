use faithea::{data::inbound::FromRequest, data::Json, handler::types::HttpHandlerError, post, request::HttpRequest, request::TryFromRequest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Stu {
    name: String,
    age: i32,
}

impl<'a> TryFromRequest<'a> for Stu {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
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
