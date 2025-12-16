#[derive(Debug, Hash, PartialEq, Eq,Clone, Copy)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
}
impl TryFrom<&str> for Method {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "get"       => Ok(Self::GET),
            "post"      => Ok(Self::POST),
            "delete"    => Ok(Self::DELETE),
            "put"       => Ok(Self::PUT),
            _           => Err(format!("{} is not a method", value)),
        }
    }
}
impl TryFrom<String> for Method {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value: &str = value.as_ref();
        value.try_into()
    }
}
impl TryFrom<&String> for Method {
    type Error = String;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        let value: &str = value.as_ref();
        value.try_into()
    }
}
