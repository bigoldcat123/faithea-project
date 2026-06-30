use faithea::MultipartData;
use faithea::data::inbound::multipart::MultiPartFile;
use faithea::data::inbound::multipart::Multipart;
use faithea::data::inbound::multipart::Part;
use faithea::data::inbound::multipart::TryFromParts;
use faithea::error::MultipartError;
use faithea::post;

#[derive(Debug)]
pub struct A {
    pub value: String,
}

impl TryFromParts for A {
    fn try_from_parts(part: Option<Vec<Part>>) -> Result<Self, MultipartError> {
        if let Some(mut part) = part
            && let Some(Part::Lit(s)) = part.pop()
        {
            Ok(Self { value: s })
        } else {
            Err(MultipartError::FieldNotExist)
        }
    }
}

#[derive(Debug,MultipartData)]
struct StuInfo {
    #[faithea(rename = "otherInfo")]
    pub other_info: A,
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: Vec<MultiPartFile>,
}

// impl faithea::data::inbound::multipart::TryFromMultipartDataMap for StuInfo {
//     fn try_from_multipart_data_map(
//         data: &mut std::collections::HashMap<String, Vec<faithea::data::inbound::multipart::Part>>,
//     ) -> Result<Self, faithea::error::MultipartError> {
//         use faithea::data::inbound::multipart::TryFromParts;
//         Ok(Self {
//             other_info: TryFromParts::try_from_parts(data.remove("otherInfo"))?,
//             name: TryFromParts::try_from_parts(data.remove("name"))?,
//             age: TryFromParts::try_from_parts(data.remove("age"))?,
//             merried: TryFromParts::try_from_parts(data.remove("merried"))?,
//             profile: TryFromParts::try_from_parts(data.remove("profile"))?,
//         })
//     }
// }

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
