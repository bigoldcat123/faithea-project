use http_server::{
    MultipartData, data::{
        Json,
        inbound::multipart::{MultiPartFile, Multipart}, outbound::StaticFile,
    }, get, handlers, post, request::static_map, res_modifiers, server::HttpServer
};
use serde::{Deserialize, Serialize};

#[post("/modifier/{name}/{age}")]
async fn search_params_and_path_params_and_json(
    name: String,
    age: usize,
    stu: Json<Stu>,
    #[search_param] account: usize,
    #[search_param] new_name: String,
) {
    println!("name: {}, age:{}, account:{}, new_name:{}",name,age,account,new_name);
    res_modifiers!(stu)
}
#[get("/**")]
async fn static_file_map() {
    static_map(&_req, "/Users/dadigua/Desktop/graduation/front-end-app").await
}

#[get("/")]
async fn get_user() {
    StaticFile("/Users/dadigua/Desktop/graduation/app/src/main.rs")
}

#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
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
    let data = data.into_inner();
    println!(
        "{:?} {:?} {:?} {:?}",
        data.age, data.name, data.profile, data.merried
    );
    "ok"
}

#[get("/")]
async fn hello_world() {
    "Hello,World"
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    HttpServer::builder()
        // .mount("/user", handlers!(get_user, multipart))
        .mount("/", handlers!(hello_world))
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
