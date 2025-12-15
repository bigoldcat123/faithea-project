#![allow(dead_code, unused)]
use std::collections::HashMap;

use chenzhonghai_app::json;
use http_server::{
    MultipartData, data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart},
        },
        outbound::StaticFile,
    }, get, handlers, post, request::{ConvertFromRefString, HttpRequest, search_param, static_map}, res_modifiers, response::HttpResponse, server::HttpServer
};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt};

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
#[derive(MultipartData, Debug)]
struct StuInfo {
    pub name: Vec<String>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: Vec<MultiPartFile>,
}

#[post("/multipart")]
async fn multipart(data: Multipart<StuInfo>) {
    let mut data = data.into_inner();
    println!("{:?}",data);
    "ok"
}

#[get("/")]
async fn hello_world() {
    "Hello,World"
}
#[get("/cookie")]
async fn cookie() {
    println!("{:?}", _req.cookies());
    "good got your cookie"
}
#[get("/optional/{age}")]
async fn optional(#[search_param]name:Option<&String>,age:u16) {
    println!("{:?} {:?}",name,age);
    "good got your optional"
}

#[post("/fromRequest")]
async fn fromRequest(stu:FromRequest<Stu>) {
    Json(stu.into_inner())
}

#[tokio::main(flavor="current_thread")]
async fn main() {
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    HttpServer::builder()
        .mount("/", handlers!(cookie, multipart, optional,fromRequest,json))
        .guard("/protected/**", async |req| {
            Ok(req)
        })
        .guard("/**", async |e| {
            Ok(e)
        })
        .build()
        .start()
        .await;
}

// guards.add("/hello/*", async |req| {
//     println!("[Guard 1] Processing request under /hello/* path");
//     Ok(req)
// });
// guards.add("/hello/asdasd", async |req| {
//     println!("[Guard 2] Processing request for exact path /hello/asdasd");
//     Ok(req)
// });
// guards.add("/**", async |req| {
//     println!("[Guard 3] Processing any request (catch-all)");
//     Ok(req)
// });
