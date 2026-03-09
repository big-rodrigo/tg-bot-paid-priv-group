use crate::{
    db::{models::Payment, DbPool},
    db_execute, db_query_as,
    error::Result,
};

pub async fn create(
    pool: &DbPool,
    user_id: i64,
    provider: &str,
    price_cents: Option<i64>,
) -> Result<i64> {
    let (id,): (i64,) = db_query_as!(pool, (i64,),
        "INSERT INTO payments (user_id, provider, price_cents, status) VALUES (?, ?, ?, 'pending') RETURNING id",
        [user_id, provider, price_cents], fetch_one)?;
    Ok(id)
}

pub async fn set_external_ref(pool: &DbPool, id: i64, external_ref: &str) -> Result<()> {
    db_execute!(pool,
        "UPDATE payments SET external_ref = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        [external_ref, id])?;
    Ok(())
}

pub async fn complete_external(
    pool: &DbPool,
    external_ref: &str,
    payload: &str,
    amount: Option<i64>,
    currency: Option<&str>,
) -> Result<Option<Payment>> {
    db_execute!(pool,
        "UPDATE payments
         SET status = 'completed',
             payload = ?,
             amount = COALESCE(?, amount),
             currency = COALESCE(?, currency),
             updated_at = CURRENT_TIMESTAMP
         WHERE external_ref = ? AND status = 'pending'",
        [payload, amount, currency, external_ref])?;

    db_query_as!(pool, Payment, "SELECT * FROM payments WHERE external_ref = ?", [external_ref], fetch_optional)
        .map_err(Into::into)
}

pub async fn get_completed_for_user(pool: &DbPool, user_id: i64) -> Result<Option<Payment>> {
    db_query_as!(pool, Payment,
        "SELECT * FROM payments WHERE user_id = ? AND status = 'completed' ORDER BY updated_at DESC LIMIT 1",
        [user_id], fetch_optional)
        .map_err(Into::into)
}

pub async fn list(pool: &DbPool, status_filter: Option<&str>) -> Result<Vec<Payment>> {
    match status_filter {
        Some(status) => db_query_as!(pool, Payment,
            "SELECT * FROM payments WHERE status = ? ORDER BY created_at DESC",
            [status], fetch_all)
            .map_err(Into::into),
        None => db_query_as!(pool, Payment,
            "SELECT * FROM payments ORDER BY created_at DESC",
            [], fetch_all)
            .map_err(Into::into),
    }
}

pub async fn get_pending_for_user(pool: &DbPool, user_id: i64) -> Result<Option<Payment>> {
    db_query_as!(pool, Payment,
        "SELECT * FROM payments WHERE user_id = ? AND status = 'pending' ORDER BY created_at DESC LIMIT 1",
        [user_id], fetch_optional)
        .map_err(Into::into)
}
