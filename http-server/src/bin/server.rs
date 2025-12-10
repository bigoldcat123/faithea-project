use http_server::{gurad::GurardTire, handler::HandlerTire, request::HttpRequest, response::HttpResponse, server::HttpServer};
use tokio::fs::File;

async fn handle1(_req:HttpRequest) -> HttpResponse {
    let mut res = HttpResponse::new();
    res.add_header(("content-length","5"));
    res.set_body(http_server::response::ResponseBody::Simple("12345".into()));
    res
}

async fn handle2(_req:HttpRequest) -> HttpResponse {
    let mut res = HttpResponse::new();
    let f = File::open("/Users/dadigua/Desktop/graduation/http-server/src/bin/server.rs").await.unwrap();
    let len = f.metadata().await.unwrap().len();
    res.add_header(("content-length",len.to_string().as_str()));
    // res.add_header(("Connection","close"));
    res.set_body(http_server::response::ResponseBody::File(f));
    res
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut handler = HandlerTire::default();
    handler.add("/hello/{abc}".to_string(), handle1);
    handler.add("/file".to_string(), handle2);
    handler.add("/".to_string(), handle2);

    let mut gurads = GurardTire::default();


    gurads.add("/hello/*".to_string(), async |req| {
        println!("哈哈哈");
        // Err(HttpResponse::not_found())
        Ok(req)
    });
    gurads.add("/hello/asdasd".to_string(), async |req| {
        println!("哈哈哈2");
        Ok(req)
    });
    gurads.add("/**".to_string(), async |req| {
        println!("哈哈哈3");
        Ok(req)
    });

    let  server = HttpServer::new("127.0.0.1:8899",handler,gurads);

    server.start().await
}
