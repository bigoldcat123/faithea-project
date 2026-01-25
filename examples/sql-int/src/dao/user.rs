use sqlx::{
    prelude::FromRow,
    types::{BigDecimal, chrono},
};

use crate::db;

#[derive(FromRow, Debug, Default)]
pub struct User {
    pub id: i32,
    pub create_time: chrono::NaiveDate,
    pub name: String,
    pub age: i32,
    pub is_active: bool,
    pub balance: BigDecimal,
}

pub struct UserDao;
impl UserDao {
    pub async fn get_one_by_id(id: i32) -> Result<User, sqlx::Error> {
        sqlx::query_as!(User, "SELECT * FROM _user WHERE id = $1", id)
            .fetch_one(db())
            .await
    }
    pub async fn insert(user: User) -> Result<User, sqlx::Error> {
        sqlx::query_as!(User, "
            INSERT INTO _user (create_time, name, age, is_active, balance)
            VALUES ($1, $2, $3, $4, $5) RETURNING *",
            user.create_time, user.name, user.age, user.is_active, user.balance)
            .fetch_one(db())
            .await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::types::{BigDecimal, chrono::{ Local}};

    use crate::{
        dao::user::{User, UserDao},
        init_db,
    };

    #[tokio::test]
    async fn insert_test() {
        init_db().await;
        let user = UserDao::insert({
            User {
                create_time:Local::now().date_naive(),
                name: "John Doe".to_string(),
                age: 30,
                is_active: true,
                balance: BigDecimal::from(10),
                ..Default::default()
            }
        }).await.unwrap();
        println!("{:?}", user);
    }
    #[tokio::test]
    async fn test_name() {
        init_db().await;
        let user = UserDao::get_one_by_id(2).await.unwrap();
        println!("{:#?}", user);
    }
}
