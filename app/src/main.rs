#![allow(unused)]
use chenzhonghai_app::json;
use http_server::{
    MultipartData,
    data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart, Part},
        },
    },
    get, handlers, post,
    request::{HttpRequest, search_param},
    server::HttpServer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
impl TryFrom<&HttpRequest> for Stu {
    type Error = String;
    fn try_from(value: &HttpRequest) -> Result<Self, Self::Error> {
        Ok(Stu {
            name: "from req".into(),
            age: 111,
        })
    }
}
#[derive(Debug)]
struct A {}
impl TryFrom<Part> for A {
    type Error = String;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

#[derive(MultipartData, Debug)]
struct StuInfo {
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: MultiPartFile,
}

#[post("/multipart")]
async fn multipart(data: Multipart<StuInfo>) {
    format!(
        "name: {:?},age: {}, merried: {:?}, profile_len: {}, profile_name:{:?}",
        data.name,
        data.age,
        data.merried,
        data.profile.data.len(),
        data.profile.file_name
    )
}

#[get("/")]
async fn hello_world() {
    "Hello,World"
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
        )
        .guard("/protected/**", async |req| Ok(req))
        .guard("/**", async |e| Ok(e))
        .build()
        .start()
        .await;
}
