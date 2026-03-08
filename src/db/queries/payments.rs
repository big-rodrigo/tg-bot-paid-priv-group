use crate::{
    db::{models::Payment, DbPool},
    error::Result,
};

pub async fn create(
    pool: &DbPool,
    user_id: i64,
    provider: &str,
    price_cents: Option<i64>,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO payments (user_id, provider, price_cents, status) VALUES (?, ?, ?, 'pending')",
    )
    .bind(user_id)
    .bind(provider)
    .bind(price_cents)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn set_external_ref(pool: &DbPool, id: i64, external_ref: &str) -> Result<()> {
    sqlx::query(
        "UPDATE payments SET external_ref = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(external_ref)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn complete_external(
    pool: &DbPool,
    external_ref: &str,
    payload: &str,
    amount: Option<i64>,
    currency: Option<&str>,
) -> Result<Option<Payment>> {
    sqlx::query(
        "UPDATE payments
         SET status = 'completed',
             payload = ?,
             amount = COALESCE(?, amount),
             currency = COALESCE(?, currency),
             updated_at = CURRENT_TIMESTAMP
         WHERE external_ref = ? AND status = 'pending'",
    )
    .bind(payload)
    .bind(amount)
    .bind(currency)
    .bind(external_ref)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE external_ref = ?")
        .bind(external_ref)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn complete_telegram(
    pool: &DbPool,
    id: i64,
    telegram_charge_id: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE payments SET status = 'completed', telegram_charge_id = ?,
         updated_at = CURRENT_TIMESTAMP WHERE id = ?",
    )
    .bind(telegram_charge_id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<Payment>> {
    sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn get_completed_for_user(pool: &DbPool, user_id: i64) -> Result<Option<Payment>> {
    sqlx::query_as::<_, Payment>(
        "SELECT * FROM payments WHERE user_id = ? AND status = 'completed' ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn list(pool: &DbPool, status_filter: Option<&str>) -> Result<Vec<Payment>> {
    match status_filter {
        Some(status) => sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments WHERE status = ? ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(pool)
        .await
        .map_err(Into::into),
        None => sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into),
    }
}

pub async fn get_pending_for_user(pool: &DbPool, user_id: i64) -> Result<Option<Payment>> {
    sqlx::query_as::<_, Payment>(
        "SELECT * FROM payments WHERE user_id = ? AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}
