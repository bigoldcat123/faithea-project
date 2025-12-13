use app::{Stu,m2,hello_world_v2,test_pathparam};
use http_server::{
    HttpHeader, data::Json, guard::GuardTire, handler::HandlerTire, res_modifiers, server::HttpServer,
};
use http_server_macro::{ get, handlers, post};

#[post("/modifier/{name}")]
async fn m(name: String, stu: http_server::data::Json<Stu>) {
    let r: Json<Stu> = Json(Stu {
        name: format!("hello da大地瓜 -> {}", name),
    });
    let mut header = HttpHeader::new();
    header.add("hello", serde_json::to_string(&stu).unwrap());
    res_modifiers!(header, r)
}
#[get("/static/**")]
async fn static_map() {

    "hello dadigua"
}


#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Set up the handler routing trie
    let mut handler = HandlerTire::default();
    handler.mount(handlers!(m,m2,hello_world_v2,test_pathparam,static_map));
    let guards = GuardTire::default();

    let server = HttpServer::new("127.0.0.1:8899", handler, guards);
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    server.start().await;
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
