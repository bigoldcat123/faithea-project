use http_server::{
    HttpHeader,
    data::Json,
    guard::GuardTire,
    handler::HandlerTire,
    request::{ConvertFromRefString, HttpRequest},
    res_modifiers,
    response::{HttpResponse, HttpResponseModifier},
    server::HttpServer,
};
use http_server_macro::get;
use serde::{Deserialize, Serialize};
use tokio::fs::File;

async fn handle_path_param(_req: HttpRequest) -> Result<HttpResponse, String> {
    let mut res = HttpResponse::new();
    res.add_header(("content-length", "5"));
    res.set_body(http_server::response::ResponseBody::Simple("12345".into()));
    Ok(res)
}

async fn handle_file(_req: HttpRequest) -> Result<HttpResponse, String> {
    let mut res = HttpResponse::new();

    let file_path = "/Users/dadigua/Desktop/graduation/http-server/src/bin/server.rs";
    let f = File::open(file_path).await.unwrap();
    let len = f.metadata().await.unwrap().len();

    res.add_header(("content-length", len.to_string().as_str()));
    res.set_body(http_server::response::ResponseBody::File(f));
    Ok(res)
}
#[derive(Serialize, Deserialize)]
struct Stu {
    name: String,
}

#[get("/modifier/{name}")]
async fn m(name: String, stu: http_server::data::Json<Stu>) {
    let r: Json<Stu> = Json(Stu {
        name: format!("hello da大地瓜 -> {}", name),
    });
    let mut header = HttpHeader::new();
    header.add("hello", serde_json::to_string(&stu).unwrap());
    res_modifiers!(header, r)
}

// async fn handle_modifier(_req: HttpRequest) -> Result<HttpResponse, String> {
//     let mut res = HttpResponse::new();

//     let res_modifier = m(
//         _req.get_pathparam("name").ok_or("err".to_string())?.convert()?,
//         (&_req).try_into()?,
//     )
//     .await;

//     res_modifier.modify(&mut res);
//     Ok(res)
// }

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Set up the handler routing trie
    let mut handler = HandlerTire::default();

    handler.add("/hello/{abc}", handle_path_param);

    handler.add("/file", handle_file);
    handler.add("/", handle_file);
    handler.add("/modifier/{name}", m_handler);
    let mut guards = GuardTire::default();
    guards.add("/hello/*", async |req| {
        println!("[Guard 1] Processing request under /hello/* path");
        Ok(req)
    });

    guards.add("/hello/asdasd", async |req| {
        println!("[Guard 2] Processing request for exact path /hello/asdasd");
        Ok(req)
    });

    guards.add("/**", async |req| {
        println!("[Guard 3] Processing any request (catch-all)");
        Ok(req)
    });

    let server = HttpServer::new("127.0.0.1:8899", handler, guards);
    println!("HTTP server starting on http://127.0.0.1:8899");
    println!("Press Ctrl+C to stop the server");
    server.start().await;
}
