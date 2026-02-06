use faithea::{
    handler::types::HttpHandlerError, request::TryFromParam,
    get,
};

#[derive(Debug)]
pub struct MyAge {
    pub age: i32,
}

impl TryFromParam<'_> for MyAge {
    fn try_from_param(value: &str) -> Result<Self, HttpHandlerError> {
        let a = value
            .parse::<i32>()
            .map_err(|_| HttpHandlerError::before_handler_invalid_param("cause"))?;
        Ok(Self { age: a })
    }
}

#[get("/pathParam/{name}/{age}")]
pub async fn path_param(name: String, age: MyAge) {
    format!("name is {}, age is {:?}", name, age)
}
