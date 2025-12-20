#![allow(unused)]
use chenzhonghai_app::json;
use chenzhonghai_app::static_file_map::file_map;
use http_server::{
    MultipartData, data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart, Part},
        },
    }, get, handlers, post, request::{HttpRequest, search_param}, res_modifiers, response, server::HttpServer
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
impl TryFrom<&mut HttpRequest> for Stu {
    type Error = String;
    fn try_from(value: &mut HttpRequest) -> Result<Self, Self::Error> {
        Ok(Stu {
            name: "from req".into(),
            age: 111,
        })
    }
}
#[derive(Debug)]
struct A {
    value:String
}
impl TryFrom<Part> for A {
    type Error = String;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        if let Part::Lit(s) = value {
            Ok(Self {
                value:s
            })
        }else {
            Err("ggg".to_string())
        }
    }
}

#[derive(MultipartData, Debug)]
struct StuInfo {
    pub other_info:A,
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: MultiPartFile,
}

#[post("/multipart")]
async fn multipart(data: Multipart<StuInfo>) {


    let mut f = tokio::fs::File::open(data.profile.temp_path.as_str()).await.unwrap();
    // let mut s = String::new();
    // f.read_to_string(&mut s).await.unwrap();
    println!("{}",f.metadata().await.unwrap().len());

    format!(
        "name: {:?},age: {}, merried: {:?}, profile_len: {}, profile_name:{:?} other_info:{:?}",
        data.name,
        data.age,
        data.merried,
        data.profile.temp_path.len(),
        data.profile.file_name,
        data.other_info
    )
}

#[get("/")]
async fn hello_world() {
    println!("哈哈哈");

    let mut c = response::cookie::Cookie::default();
    c.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    c.insert("Access-Control-Allow-Methods".to_string(), "GET, POST, PUT, DELETE".to_string());
    c.insert("Access-Control-Allow-Headers".to_string(), "*".to_string());
    c.insert("Access-Control-Allow-Credentials".to_string(), "true".to_string());
    res_modifiers!("Hello,World",c)
}
#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}

#[get("/pathParam/{name}/{age}")]
async fn pathParam(name: String, age: i32) {
    format!("name is {}, age is {}", name, age)
}

#[get("/searchParam")]
async fn search_param(#[search_param] name: &String, #[search_param] age: Option<i32>) {
    format!("name is {} and age is {:?}", name, age)
}

#[post("/fromRequest")]
async fn fromRequest(stu: FromRequest<Stu>) {
    Json(stu.into_inner())
}
//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    HttpServer::builder()
        .mount(
            "/",
            handlers!(
                hello_world,
                cookie,
                multipart,
                pathParam,
                search_param,
                fromRequest,
                json
            ),
        ).mount("/static", handlers!(file_map))
        .cors()
        .guard("/protected/**", async |req| Ok(req))
        .guard("/**", async |e| Ok(e))
        .build()
        .start()
        .await;
}
