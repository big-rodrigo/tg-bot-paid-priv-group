use crate::{
    db::{models::User, DbPool},
    db_execute, db_query_as,
    error::{AppError, Result},
};

pub async fn upsert(
    pool: &DbPool,
    telegram_id: i64,
    username: Option<&str>,
    first_name: &str,
    last_name: Option<&str>,
) -> Result<User> {
    db_execute!(pool,
        "INSERT INTO users (telegram_id, username, first_name, last_name)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(telegram_id) DO UPDATE SET
             username = excluded.username,
             first_name = excluded.first_name,
             last_name = excluded.last_name",
        [telegram_id, username, first_name, last_name])?;

    get_by_telegram_id(pool, telegram_id)
        .await?
        .ok_or_else(|| AppError::Other("user not found after upsert".into()))
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<User>> {
    db_query_as!(pool, User, "SELECT * FROM users WHERE id = ?", [id], fetch_optional)
        .map_err(Into::into)
}

pub async fn get_by_telegram_id(pool: &DbPool, telegram_id: i64) -> Result<Option<User>> {
    db_query_as!(pool, User, "SELECT * FROM users WHERE telegram_id = ?", [telegram_id], fetch_optional)
        .map_err(Into::into)
}

pub async fn list(pool: &DbPool, page: i64, limit: i64) -> Result<Vec<User>> {
    let offset = (page - 1).max(0) * limit;
    db_query_as!(pool, User, "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?", [limit, offset], fetch_all)
        .map_err(Into::into)
}

/// Reset a user's registration progress: clear answers, cancel pending payments,
/// and delete the user_registration row so they can /start fresh.
pub async fn reset_registration(pool: &DbPool, user_id: i64) -> Result<()> {
    db_execute!(pool, "DELETE FROM answers WHERE user_id = ?", [user_id])?;

    db_execute!(pool, "UPDATE payments SET status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE user_id = ? AND status = 'pending'", [user_id])?;

    db_execute!(pool, "DELETE FROM user_registration WHERE user_id = ?", [user_id])?;

    Ok(())
}

/// Fully unregister a user: reset registration, revoke all unused invite links,
/// and delete invite link records.
pub async fn unregister(pool: &DbPool, user_id: i64) -> Result<()> {
    reset_registration(pool, user_id).await?;

    db_execute!(pool, "UPDATE invite_links SET revoked_at = CURRENT_TIMESTAMP WHERE user_id = ? AND used_at IS NULL AND revoked_at IS NULL", [user_id])?;

    db_execute!(pool, "DELETE FROM invite_links WHERE user_id = ?", [user_id])?;

    db_execute!(pool, "DELETE FROM payments WHERE user_id = ?", [user_id])?;

    Ok(())
}
