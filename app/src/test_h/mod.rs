use faithea::{handlers, server::HandlerModifier};

use crate::test_handler::json::json_test;

pub mod json;
pub mod hello_world;

pub fn test_handlers() -> Vec<HandlerModifier> {
    handlers!(json_test)
}
