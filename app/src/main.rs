#![allow(unused)]
use chenzhonghai_app::json;
use chenzhonghai_app::static_file_map::file_map;
use http_server::{
    HeaderMap, MultipartData, TryConvertFrom, data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart, Part},
        },
    }, get, handler::FuError, handlers, header::{ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN}, post, request::{HttpRequest, search_param}, res_modifiers, response::{self, cors::CORS}, server::{HttpServer}
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
impl TryFrom<&mut HttpRequest> for Stu {
    type Error = FuError;
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
    type Error = FuError;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        if let Part::Lit(s) = value {
            Ok(Self {
                value:s
            })
        }else {
            Err(Box::new("ggg") as FuError)
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

    res_modifiers!("Hello,World",CORS)
}
#[get("/cookie")]
async fn cookie() {
    format!("{:?}", _req.cookies())
}
#[derive(Debug)]
struct MyAge {
    age:i32
}
impl TryConvertFrom<Option<&String>> for MyAge {
    fn try_convert_from(value: Option<&String>) -> Result<Self, FuError> {
        if let Some(value) = value {
            let a = value.parse::<i32>().map_err(|e| Box::new(e.to_string()) as FuError)?;
            Ok(Self { age: a })
        }else {
            Err(Box::new("e") as FuError)
        }
    }
}
#[get("/pathParam/{name}/{age}")]
async fn pathParam(name: String, age: MyAge) {
    format!("name is {}, age is {:?}", name, age)
}

#[get("/searchParam")]
async fn search_param(#[search_param] name: &String, #[search_param] age: Option<MyAge>) {
    format!("name is {} and age is {:?}", name, age)
}

#[post("/fromRequest")]
async fn fromRequest(stu: FromRequest<Stu>) {
    Json(stu.into_inner())
}
//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {
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
        // .tls("/Users/dadigua/Desktop/graduation/key.pem", "/Users/dadigua/Desktop/graduation/cert.pem")
        // .h2()
        // .host("0.0.0.0")
        // .port(443)
        .build()
        .run()
        .await
        .unwrap();

}
