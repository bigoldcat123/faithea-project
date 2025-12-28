// #![allow(unused)]
use chenzhonghai_app::json;
use chenzhonghai_app::static_file_map::file_map;
use faithea::{
    MultipartData, TryConvertFrom,
    data::{
        Json,
        inbound::{
            FromRequest,
            multipart::{MultiPartFile, Multipart, Part},
        },
    },
    get,
    handler::FuError,
    handlers,
    post,
    request::HttpRequest,
    res_modifiers,
    response::cors::CORS,
    server::HttpServer,
};
use serde::{Deserialize, Serialize};
use sqlx::mysql::MySqlPoolOptions;

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
    value: String,
}
impl TryFrom<Part> for A {
    type Error = FuError;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        if let Part::Lit(s) = value {
            Ok(Self { value: s })
        } else {
            Err(Box::new("ggg") as FuError)
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
        .map(|x| x.file_name.clone())
        .collect::<Vec<_>>();
    // let mut file = tokio::fs::File::open(&data.profile[0].temp_path).await.unwrap();
    // let mut target =  tokio::fs::File::create(format!("/Users/dadigua/Desktop/graduation/{}",&data.profile[0].file_name.as_ref().unwrap())).await.unwrap();
    // tokio::io::copy(&mut file, &mut target).await.unwrap();
    println!("哈哈哈");

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
#[derive(Debug)]
struct MyAge {
    age: i32,
}
impl TryConvertFrom<Option<&String>> for MyAge {
    fn try_convert_from(value: Option<&String>) -> Result<Self, FuError> {
        if let Some(value) = value {
            let a = value
                .parse::<i32>()
                .map_err(|e| Box::new(e.to_string()) as FuError)?;
            Ok(Self { age: a })
        } else {
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
    let a = hell().await.unwrap();
    // let a = tokio::spawn(async {
    //     a
    // }).await.unwrap();
    format!("name is {} and age is {:?}", name, age)
}

#[derive(Debug)]
struct Ae{}
async fn hell() -> Result<String,Ae>{
    Err(Ae{})
}

#[post("/fromRequest")]
async fn fromRequest(stu: FromRequest<Stu>) {
    Json(stu.into_inner())
}
//(flavor = "current_thread")
#[tokio::main(flavor = "current_thread")]
async fn main() {

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
