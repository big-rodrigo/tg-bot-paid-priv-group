use crate::{
    db::{models::User, DbPool},
    error::{AppError, Result},
};

pub async fn upsert(
    pool: &DbPool,
    telegram_id: i64,
    username: Option<&str>,
    first_name: &str,
    last_name: Option<&str>,
) -> Result<User> {
    sqlx::query(
        "INSERT INTO users (telegram_id, username, first_name, last_name)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(telegram_id) DO UPDATE SET
             username = excluded.username,
             first_name = excluded.first_name,
             last_name = excluded.last_name",
    )
    .bind(telegram_id)
    .bind(username)
    .bind(first_name)
    .bind(last_name)
    .execute(pool)
    .await?;

    get_by_telegram_id(pool, telegram_id)
        .await?
        .ok_or_else(|| AppError::Other("user not found after upsert".into()))
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn get_by_telegram_id(pool: &DbPool, telegram_id: i64) -> Result<Option<User>> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE telegram_id = ?")
        .bind(telegram_id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn list(pool: &DbPool, page: i64, limit: i64) -> Result<Vec<User>> {
    let offset = (page - 1).max(0) * limit;
    sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
