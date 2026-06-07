use std::{net::TcpListener, time::Duration};

use chenzhonghai_app::test_handler::test_handlers;
use faithea::server::HttpServer;
use reqwest::{
    Client, StatusCode,
    header::{ACCESS_CONTROL_ALLOW_ORIGIN, COOKIE, LOCATION},
    multipart::Form,
    redirect::Policy,
};
use serde_json::{Value, json};
use tokio::{task::JoinHandle, time::sleep};

struct TestServer {
    base_url: String,
    task: JoinHandle<()>,
}

impl TestServer {
    async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("reserve a test port");
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let server = HttpServer::builder()
            .host("127.0.0.1")
            .port(port)
            .mount("/", test_handlers())
            .build();
        let task = tokio::spawn(async move {
            let _ = server.run().await;
        });
        let base_url = format!("http://127.0.0.1:{port}");

        for _ in 0..50 {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                return Self { base_url, task };
            }
            sleep(Duration::from_millis(500)).await;
        }

        panic!("test server did not start on {base_url}");
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[tokio::test]
async fn hello_world_returns_body_and_cors_headers() {
    let server = TestServer::start().await;
    let response = Client::new().get(server.url("/")).send().await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert_eq!(response.text().await.unwrap(), "Hello,World");
}

#[tokio::test]
async fn path_search_and_cookie_handlers_read_request_values() {
    let server = TestServer::start().await;
    let client = Client::new();

    let path_response = client
        .get(server.url("/pathParam/Alice/21"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(path_response, "name is Alice, age is MyAge { age: 21 }");

    let search_response = client
        .get(server.url("/searchParam?Name=Alice&age=21"))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(search_response, "name is Alice and age is Some(\"21\")");

    let cookie_response = client
        .get(server.url("/cookie"))
        .header(COOKIE, "session=abc; theme=dark")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(cookie_response.contains("\"session\": \"abc\""));
    assert!(cookie_response.contains("\"theme\": \"dark\""));
}

#[tokio::test]
async fn json_and_from_request_handlers_return_json() {
    let server = TestServer::start().await;
    let client = Client::new();
    let student = json!({
        "name": "Alice",
        "age": 21,
        "contact": {
            "email": "alice@example.com",
            "phones": ["123456"],
            "address": {
                "street": "Main Street",
                "city": "Shanghai",
                "coords": { "lat": 31.2304, "lng": 121.4737 }
            }
        },
        "scores": [{
            "subject": "Rust",
            "score": 98.5,
            "rank": 1,
            "detail": { "daily": 99.0, "midterm": 98.0, "final_exam": 98.5 }
        }],
        "club": null,
        "meta": {
            "created_at": "2026-06-07",
            "updated_at": "2026-06-07",
            "extra": { "graduated": true }
        }
    });

    let echoed: Value = client
        .post(server.url("/json"))
        .json(&student)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(echoed, student);

    let from_request: Value = client
        .post(server.url("/fromRequest"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(from_request, json!({ "name": "from req", "age": 111 }));
}

#[tokio::test]
async fn custom_redirect_and_multipart_handlers_modify_responses() {
    let server = TestServer::start().await;
    let client = Client::builder().redirect(Policy::none()).build().unwrap();

    let custom = client.get(server.url("/custom_res2")).send().await.unwrap();
    assert_eq!(custom.headers()["some-custom-header"], "Hello");
    assert_eq!(custom.headers()[ACCESS_CONTROL_ALLOW_ORIGIN], "*");
    assert_eq!(
        custom.json::<Value>().await.unwrap(),
        json!({ "name": "something~" })
    );

    let redirect = client.get(server.url("/redirect")).send().await.unwrap();
    assert_eq!(redirect.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(redirect.headers()[LOCATION], "https://www.baidu.com");

    let form = Form::new()
        .text("otherInfo", "extra")
        .text("name", "Alice")
        .text("name", "Bob")
        .text("age", "21")
        .text("merried", "true");
    let multipart = client
        .post(server.url("/multipart"))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(multipart.status(), StatusCode::OK);
    let multipart_body = multipart.text().await.unwrap();
    assert!(multipart_body.contains("name: [\"Alice\", \"Bob\"]"));
    assert!(multipart_body.contains("age: 21"));
    assert!(multipart_body.contains("merried: Some(true)"));
    assert!(multipart_body.contains("A { value: \"extra\" }"));
}
