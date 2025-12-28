# TODOS
-  a basic http server ✅
-  handler guard ✅
-  handler shoule be `(req) => Resule(res,err)` ✅
-  clear all unwrap!
-  add method route support! ✅
-  shared values
-  using Builder to create server ✅
-  File Transfor ✅
-  static mapping~ ✅
-  basic httpserver ✅
-  dynamic route matching ✅
-  suppor for json inbound and outbound ✅
-  pathparam ✅
-  add `_req:HttpRequest` param for handler! ✅
-  add `mount('/',handlers!(..))`✅
-  add error information for dynamic route defination ✅
-  implememt `HttpResponseModifier` for some basic types 👷
-  serachParam ✅
-  many guard could share one route!✅
-  multipart!!!✅
-  multipart Option support ✅
-  merge macro and lib together ✅
-  add cookieModifier
-  add cookie access to request ✅
-  add new struct `FromRequest`, anything but searchParam,pathParams,json, and multiPart showing in the args of a handler, can be parsed from request ✅
-  optimise `ConvertFromRefString` to `TryConvertFrom` and `TryConvertInto` ✅
-  refactor multipart file.. save every part as file, and only keep the path to that file, when access the field, just read the file again and process parsing.✅
    1. using fixed buff to parse the html body. ✅
    2. save every part to file,and keep the file name. ✅
    3. when access using path to access it!  ✅
    > things to change `TryFromMultipartDataMap` `Part` ✅

-  support `Option` for **searchParam** in handler args✅
-  Error!!! ✅
-  add vec support for multipart ✅
-  Tls ✅
-  http2 ✅
-  WebSocket Support 💼

- impl `deref` for `Json` ✅
- add comment for `static_map` ✅
- static_map need urldecode!
- improve `handlers` macro


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
 
> the type of field shoule impl `TryFrom<Part>`

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
impl TryFrom<&mut HttpRequest> for Stu {
    type Error = String;
    fn try_from(value: &mut HttpRequest) -> Result<Self, Self::Error> {
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




# Tips
1. make your type **compatible** with searchParam and **pathParam**
```rust
impl TryConvertFrom<Option<&String>> for String {
    fn try_convert_from(value: Option<&String>) -> Result<Self, FuError> {
        if let Some(value) = value {
            Ok(value.to_string())
        } else {
            Err("missing value".into())
        }
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
impl TryFrom<Part> for A {
    type Error = FuError;
    fn try_from(value: Part) -> Result<Self, Self::Error> {
        Ok(Self{})
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
