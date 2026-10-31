use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Database migration error: {0}")]
    Migration(String),
}

pub type Result<T> = std::result::Result<T, DbError>;
