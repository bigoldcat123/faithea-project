use faithea::{data::Json, get};
use crate::{service::user_service::UserService};

#[get("/users/{id}")]
pub async fn get_user(id: i32) {
    let user = UserService::get_one_by_id(id).await;
    if let Ok(user) = user {
        Json(user)
    }else {
        Json(Default::default())
    }
}
