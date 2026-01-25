use std::sync::OnceLock;

use sqlx::{Pool, Postgres};
pub mod dao;
pub mod service;
pub mod handlers;

pub static DATABASE: OnceLock<Pool<Postgres>> = OnceLock::new();

pub async fn init_db() {
    let pool = sqlx::PgPool::connect(
        std::env::var("POSTGRE_DATABASE_URL")
            .expect("NO POSTGRE_DATABASE_URL")
            .as_str(),
    )
    .await
    .expect("CAN NOT CONNECT TO DATABASE");
    DATABASE.set(pool).expect("DATABASE already initialized");
}

pub fn db() -> &'static Pool<Postgres> {
    DATABASE.get().expect("DATABASE not initialized")
}
