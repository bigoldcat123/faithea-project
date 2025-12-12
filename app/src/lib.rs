use http_server::{HttpHeader, data::Json, res_modifiers};
use http_server_macro::{get};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize,Debug)]
pub struct Stu {
    pub name: String,
}


#[get("/hello/{name}")]
pub async fn m2(name: String, stu: http_server::data::Json<Stu>) {
    let r: Json<Stu> = Json(Stu {
        name: format!("hello da大地瓜 -> {}", name),
    });
    let mut header = HttpHeader::new();
    header.add("hello", serde_json::to_string(&stu).unwrap());
    res_modifiers!(header, r)
}
#[get("/chenzhonghai/{name}")]
pub async fn hello_world_v2(name: String, stu: http_server::data::Json<Stu>) {
    let r: Json<Stu> = Json(Stu {
        name: format!("hello da大地瓜 -> {}", name),
    });
    let mut header = HttpHeader::new();
    header.add("hello", serde_json::to_string(&stu).unwrap());
    res_modifiers!(header, r)
}


#[get("/path/{name}/{age}")]
pub async fn test_pathparam(name: String, stu:Json<Stu>,age:usize) {
    let r: Json<Stu> = Json(Stu {
        name: format!("hello my name is {} and i am {} years old", name,age),
    });
    println!("{:?}",stu);
    r
}
