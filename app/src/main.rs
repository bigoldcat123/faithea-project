#![allow(dead_code,unused)]
use http_server::{
    MultipartData,
    data::{
        Json,
        inbound::{FromRequest, multipart::{MultiPartFile, Multipart}},
        outbound::StaticFile,
    },
    get, handlers, post,
    request::{ConvertFromRefString, HttpRequest, TryConvertInto, static_map},
    res_modifiers,
    server::HttpServer,
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
        Ok(Stu { name: "from req".into(), age: 111 })
    }
}
#[derive(MultipartData, Debug)]
struct StuInfo {
    pub name: String,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: MultiPartFile,
}

#[post("/multipart")]
async fn multipart(data: Multipart<StuInfo>) {
    let mut data = data.into_inner();
    let a:Result<i32, std::io::Error> = Ok(100);
    "ok"
}

#[get("/")]
async fn hello_world() {
    "Hello,World"
}
#[get("cookie")]
async fn cookie() {
    println!("{:?}",_req.cookies());
    ""
}
#[tokio::main]
async fn main() {
    let a = &"".to_string();

    let b:Option<i32> = a.try_convert_into().unwrap();
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    HttpServer::builder()
        .mount("/", handlers!(cookie, multipart))
        .guard("/**", async |e| {
            println!("new req -> ");
            Ok(e)
        })
        .guard("/**", async |e| {
            println!("new req2 -> ");
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
