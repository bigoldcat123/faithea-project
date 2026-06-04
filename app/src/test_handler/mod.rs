use faithea::{handlers, server::HandlerModifier};

pub mod cookie;
pub mod custom_res;
pub mod from_request;
pub mod hello_world;
pub mod json;
pub mod multipart;
pub mod path_param;
pub mod redirect;
pub mod search_param;
pub mod stream;

// Re-export all handlers
pub use cookie::cookie;
pub use from_request::from_request;
pub use hello_world::hello_world;
pub use json::json_test;
pub use multipart::multipart;
pub use path_param::path_param;
pub use redirect::redirect;
pub use search_param::search_param;
pub use stream::stream;

use crate::test_handler::custom_res::{custom_res, custom_res2};

pub fn test_handlers() -> Vec<HandlerModifier> {
    handlers!(
        json_test,
        hello_world,
        cookie,
        multipart,
        path_param,
        search_param,
        from_request,
        redirect,
        stream,
        custom_res,
        custom_res2
    )
}
