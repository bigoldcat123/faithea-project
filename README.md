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




# Tips
make your type **compatible** with searchParam and **pathParam**
```rust
pub trait ConvertFromRefString<'a, O> {
    fn convert(self) -> Result<O, String>;
}
```
