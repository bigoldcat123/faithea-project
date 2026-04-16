use std::collections::HashMap;

use faithea::{post};
use faithea::data::Json;
use serde::{Deserialize, Serialize};


// 1. 最外层：学生档案
#[derive(Debug, Serialize, Deserialize)]
struct StuData {
    name: String,
    age: i32,
    contact: Contact,
    scores: Vec<Score>,
    club: Option<Club>,
    meta: Meta,
}

// 2. 联系方式
#[derive(Debug, Serialize, Deserialize)]
struct Contact {
    email: String,
    phones: Vec<String>,
    address: Address,
}

// 3. 地址
#[derive(Debug, Serialize, Deserialize)]
struct Address {
    street: String,
    city: String,
    coords: Coords, // 经纬度
}

#[derive(Debug, Serialize, Deserialize)]
struct Coords {
    lat: f64,
    lng: f64,
}

// 4. 单科成绩
#[derive(Debug, Serialize, Deserialize)]
struct Score {
    subject: String,
    score: f32,
    rank: u32,
    detail: Detail,
}

#[derive(Debug, Serialize, Deserialize)]
struct Detail {
    daily: f32,
    midterm: f32,
    final_exam: f32,
}

// 5. 社团
#[derive(Debug, Serialize, Deserialize)]
struct Club {
    name: String,
    role: String,
    join_date: String,
    activities: Vec<Activity>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Activity {
    title: String,
    date: String,
    hours: u8,
    tags: Vec<String>,
}

// 6. 系统元数据
#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    created_at: String,
    updated_at: String,
    extra: HashMap<String, serde_json::Value>, // 任意扩展字段
}
#[post("/json")]
async fn json_test(stu:Json<StuData>) {
    stu
}
