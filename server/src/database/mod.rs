pub mod models;
pub mod repository;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::path::Path;

pub type DbPool = SqlitePool;

pub async fn init_database(path: &Path) -> crate::Result<DbPool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
    
    let pool = SqlitePool::connect_with(options).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| crate::OblivionError::Database(e.into()))?;
    
    Ok(pool)
}