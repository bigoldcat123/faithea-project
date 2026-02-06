// #![allow(unused)]
use chenzhonghai_app::static_file_map::file_map;
use chenzhonghai_app::{json, ws::ws};
use faithea::data::inbound::multipart::TryFromPart;
use faithea::request::{TryFromParam, TryFromRequest};
use faithea::response::redirect::Redirect;
use faithea::{
    MultipartData, data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart, Part},
        },
    },
    get,
    handler::types::HttpHandlerError,
    handlers, post,
    request::HttpRequest,
    res_modifiers,
    response::cors::CORS,
    server::HttpServer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
impl <'a> TryFromRequest<'a> for Stu {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        Ok(Stu {
            name: "from req".into(),
            age: 111,
        })
    }
}

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
async fn multipart(data: Multipart<StuInfo>) {
    let f = data
        .profile
        .iter()
        .map(|x| (x.file_name.clone(),x.temp_path.clone()))
        .collect::<Vec<_>>();
    // let mut file = tokio::fs::File::open(&data.profile[0].temp_path).await.unwrap();
    // let mut target =  tokio::fs::File::create(format!("/Users/dadigua/Desktop/graduation/{}",&data.profile[0].file_name.as_ref().unwrap())).await.unwrap();
    // tokio::io::copy(&mut file, &mut target).await.unwrap();
    assert_eq!(data.profile.len(),16);
    assert_eq!(data.name.len(),2);
    assert_eq!(data.other_info.value,"asd");
    format!(
        "name: {:?},age: {}, merried: {:?}, other_info:{:?},profile_len: {:?},  ",
        data.name, data.age, data.merried, data.other_info, f
    )
}

#[get("/")]
async fn hello_world() {
    res_modifiers!("Hello,World", CORS)
}
#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
#[get("/redirect")]
async fn redirect() {
    println!("redirect");
    Redirect("https://localhost:443/")
}
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
async fn pathParam(name: String, age: MyAge) {
    format!("name is {}, age is {:?}", name, age)
}

#[get("/searchParam")]
async fn search_param(#[search_param] name: &str, #[search_param] age: Option<String>) {
    format!("name is {} and age is {:?}", name, age)
}

#[post("/fromRequest")]
async fn fromRequest(stu: FromRequest<Stu>) {
    Json(stu.into_inner())
}

//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {
    env_logger::init();
    let r = HttpServer::builder()
        .mount(
            "/",
            handlers!(
                hello_world,
                cookie,
                multipart,
                pathParam,
                search_param,
                fromRequest,
                json,
                redirect,
                chenzhonghai_app::util::inner_util::util2
            ),
        )
        .mount("/static", handlers!(file_map))
        .cors()
        .guard("/protected/**", async |req| Ok(req))
        .guard("/**", async |e| {
            // println!("{e:?}",);
            Ok(e)
        })
        .websocket("/ws/{name}", ws)
        .globale_error_handler(async |e: faithea::error::Error| {
            res_modifiers!(format!("some error~~ {:?}", e))
        })
        // .tls(
        //     "/Users/dadigua/Desktop/graduation/key.pem",
        //     "/Users/dadigua/Desktop/graduation/cert.pem",
        // )
        // .h2()
        // .host("0.0.0.0")
        // .port(443)
        .build()
        .run()
        .await;
    println!("{:?}", r);
}
