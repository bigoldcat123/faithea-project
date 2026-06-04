use faithea::MultipartData;
use faithea::data::inbound::multipart::MultiPartFile;
use faithea::data::inbound::multipart::Multipart;
use faithea::data::inbound::multipart::Part;
use faithea::data::inbound::multipart::TryFromPart;
use faithea::handler::types::HttpHandlerError;
use faithea::post;

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
    #[faithea(rename = "otherInfo")]
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
    format!(
        "name: {:?},age: {}, merried: {:?}, other_info:{:?},file_info: {:?},  ",
        data.name, data.age, data.merried, data.other_info, f
    )
}
