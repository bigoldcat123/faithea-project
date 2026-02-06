use faithea::{
    data::inbound::multipart::{MultiPartFile, Multipart, Part, TryFromPart},
    handler::types::HttpHandlerError,
    MultipartData, post,
};

#[derive(Debug)]
pub struct A {
    pub value: String,
}

impl TryFromPart for A {
    fn try_from_part(part: Part) -> Result<Self, HttpHandlerError> {
        if let Part::Lit(s) = part {
            Ok(Self { value: s })
        } else {
            Err(HttpHandlerError::before_handler_incompatible_request_body_type())
        }
    }
}

#[derive(MultipartData, Debug)]
struct StuInfo {
    pub other_info: A,
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: Vec<MultiPartFile>,
}

#[post("/multipart")]
pub async fn multipart(data: Multipart<StuInfo>) {
    let f = data
        .profile
        .iter()
        .map(|x| (x.file_name.clone(), x.temp_path.clone()))
        .collect::<Vec<_>>();
    // assert_eq!(data.profile.len(), 16);
    // assert_eq!(data.name.len(), 2);
    // assert_eq!(data.other_info.value, "asd");
    format!(
        "name: {:?},age: {}, merried: {:?}, other_info:{:?},profile_len: {:?},  ",
        data.name, data.age, data.merried, data.other_info, f
    )
}
