use serde::Serialize;

use crate::{dao::user::{User, UserDao}};

pub struct UserService;

#[derive(Serialize,Default)]
pub struct UserDto{
    pub id: i32,
    pub create_time: String,
    pub name: String,
    pub age: i32,
    pub is_active: bool,
    pub balance: String,
}
impl From<User> for UserDto{
    fn from(value: User) -> Self {
        UserDto{
            id: value.id,
            create_time: value.create_time.format("%Y-%m-%d").to_string(),
            name: value.name,
            age: value.age,
            is_active: value.is_active,
            balance: value.balance.to_string(),
        }
    }
}

impl UserService {
    pub async fn get_one_by_id(id: i32) -> Result<UserDto, sqlx::Error> {
        UserDao::get_one_by_id(id).await.map(Into::into)
    }
}
