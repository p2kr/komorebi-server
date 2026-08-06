use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;
use tracing::info;

use crate::core::get_db_path;

pub mod user_repo;

pub async fn init_db() -> SqlitePool {
    let db_path = get_db_path();
    let path_str = db_path.to_str().expect("unable to find db path");

    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path_str))
        .expect("invalid db url")
        .pragma("foreign_keys", "ON")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("unable to connect to db pool");

    let schema_sql = include_str!("../../assets/schema.sql");
    sqlx::raw_sql(schema_sql)
        .execute(&pool)
        .await
        .expect("failed to execute database schema migrations");

    info!("database initialized successfully at {:?}", db_path);

    pool
}
