# Example
1. Hello World
```rust
#[get("/")]
async fn hello_world() {
    "Hello,World"
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    HttpServer::builder()
        .mount("/", handlers!(hello_world))
        .build()
        .start()
        .await;
}
```
2. static file mapping
```rust
#[get("/**")]
async fn static_file_map() {
    static_map(&_req, "/path/to/directory").await
}
```
2. return a file 
```rust
#[get("/file")]
async fn file() {
    StaticFile("/path/to/file")
}
```
3. search_param eg. `/searchParam?name=hello&age=100`
```rust
#[get("/searchParam")]
async fn search_param(
    #[search_param] name: usize,
    #[search_param] age: String,
) {
    println!("name: {}, age:{}, }",name,age,);
    "good"
}
```
4. path_param
```rust
#[get("/pathParam/{name}/{age}")]
async fn path_params(
    name: String,
    age: usize,
) {
    println!("name: {}, age:{}",name,age);
    res_modifiers!("")
}
```
5. Json body of request and response
```rust
// Serialize for request body
// Deserialize for response body
#[derive(Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
#[post("/json")]
async fn search_params_and_path_params_and_json(
    stu: Json<Stu>,
) {
    stu
}
```
5. multipart form data

 derive from `MultipartData`
 
> the type of field shoule impl `TryFromPart`

```rust
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
```
6. add guard 
guard will execute before the handlers

```rust 
async fn guard_ok(req:HttpRequest) -> Result<HttpRequest,HttpResponse> {
  Ok(req)
}
async fn guard_err(req:HttpRequest) -> Result<HttpRequest,HttpResponse> {
  Err(HttpResponse::not_found())
}

HttpServer::builder()
    .mount("/", handlers!(hello_world))
    .guard("/**", async |e:HttpRequest| {
        println!("new req -> ");
        Ok(e)
    })
    .guard("/a",guard_ok)
    .guard("/b",guard_err)
    .build()
    .start()
    .await;
```

2. make anything converted from request
```rust
#[derive(Debug, Serialize, Deserialize)]
struct Stu {
    name: String,
    age: i32,
}
impl <'a> TryFromRequest<'a> for Stu {
    fn try_from_request(_req: &'a mut HttpRequest) -> Result<Self, HttpHandlerError> {
        Ok(Stu {
            name: "from req".into(),
            age: 111,
        })
    }
}
#[post("/fromRequest")]
async fn fromRequest(stu:FromRequest<Stu>) {

    serde_json::to_string(&stu.into_inner()).unwrap()
}
```

2. websocket
```rust
use std::{collections::HashMap, sync::LazyLock};

use bytes::Bytes;
use faithea::{request::HttpRequest, websocket::{data::WebSocketDataPayLoad, socket::WebSocket}};
use serde::{Deserialize, Serialize};
use tokio::sync::{
    Mutex,
    mpsc::Sender,
};

static WS_SENDERS: LazyLock<Mutex<HashMap<String, Sender<WebSocketDataPayLoad>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Serialize, Deserialize)]
struct WsDataMessage {
    r#type: String,
    to: String,
    from: String,
    content: String,
}

pub async fn ws(
    websocket: WebSocket,
    req: HttpRequest,
) {
    let  (mut r,s) = websocket.split();
    let name = req.get_pathparam("name").unwrap();
    {
        let mut map = WS_SENDERS.lock().await;
        map.insert(name.clone(), s.clone());
    }
    while let Some(msg) = r.recv().await {
        let data = serde_json::from_slice::<WsDataMessage>(msg.as_bytes()).unwrap();
        let map = WS_SENDERS.lock().await;
        if let Some(sender) = map.get(&data.to) {
            let a:Bytes = serde_json::to_vec(&data).unwrap().into();
            sender.send(WebSocketDataPayLoad::new(a)).await.unwrap();
        }
    }
}
// main builder
        .websocket("/ws/{name}",ws)
```

2. global error handler
```rust
        .globale_error_handler(async |e:faithea::error::Error|
            res_modifiers!(format!("some error~~ {:?}",e))
        )
```

# Tips
1. make your type **compatible** with searchParam and **pathParam**
```rust

#[derive(Debug)]
pub struct MyAge {
    pub age: i32,
}
impl TryFromParam for MyAge {
    fn try_from_param(value: &String) -> Result<Self, HttpHandlerError> {
        let a = value
            .parse::<i32>()
            .map_err(|_| HttpHandlerError::before_handler_invalid_param("cause"))?;
        Ok(Self { age: a })
    }
}
```
2. make your struct **compatible** with returning from handler.
implememt `HttpResponseModifier` for your struct
```rust
pub trait HttpResponseModifier {
    fn modify<'a>(
        &'a self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>>;
}
impl HttpResponseModifier for MyStruct {
    fn modify<'a>(
        &'a self,
        res: &'a mut HttpResponse,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), FuError>> + 'a + Send + Sync>> {
        Box::pin(async move {
            /// your code to modify response
            Ok(())
        })
    }
}
```

3. using `modifiers!()` to return multiple modifier

4. you can have an access to HttpRequest in any handler through `_req`


5. make your struct be the field in a multipartData
just impl the std TryFrom<Part> with error = String
```rust
#[derive(Debug)]
struct A{

}
impl TryFromPart for A {
    fn try_from_part(part: Part) -> Result<Self, HttpHandlerError> {
        if let Part::Lit(s) = part {
            Ok(Self { value: s })
        } else {
            Err(HttpHandlerError::before_handler_incompatible_request_body_type())
        }
    }
}

#[derive(MultipartData, Debug)]
struct StuInfo {
    pub name: Vec<A>,
    pub age: i32,
    pub merried: Option<bool>,
    pub profile: Vec<MultiPartFile>,
}
```
