use faithea::get;

#[get("/searchParam")]
pub async fn search_param(#[search_param] name: &str, #[search_param] age: Option<String>) {
    format!("name is {} and age is {:?}", name, age)
}
