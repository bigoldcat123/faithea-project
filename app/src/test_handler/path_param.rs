use faithea::{
    get,
    request::{ConvertError, TryFromParam, error::ParseHandlerParamError},
};

#[derive(Debug)]
pub struct MyAge {
    pub age: i32,
}

impl TryFromParam<'_> for MyAge {
    fn try_from_param(value: &str) -> Result<Self, ParseHandlerParamError> {
        let a = value.parse::<i32>().map_err(|_| ConvertError {
            from: value.into(),
            to: "MyAge".into(),
        })?;
        Ok(Self { age: a })
    }
}

#[get("/pathParam/{name}/{age}")]
pub async fn path_param(name: String, age: MyAge) {
    format!("name is {}, age is {:?}", name, age)
}
