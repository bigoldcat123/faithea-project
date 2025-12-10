
use http_server::{
    HttpHeader, handler::Handler, request::{HttpReqLine, HttpRequest}, response::HttpResponse
};


async fn f2(_: HttpRequest) -> HttpResponse {
    HttpResponse::new()
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut h = Handler::default();
    h.add("url".to_string(), async |_| HttpResponse::new());
    h.add("url2".to_string(), f2);

    if let Some(f) = h.get("url") {
        let res = f(HttpRequest::new(
            HttpReqLine::parse("asd asd asd").unwrap(),
            HttpHeader::new(),
            None,
        ))
        .await;
        println!("{:?}", res);
    }
}
