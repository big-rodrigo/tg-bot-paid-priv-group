use crate::{
    db::{models::InviteLink, DbPool},
    error::Result,
};

pub async fn create(
    pool: &DbPool,
    user_id: i64,
    group_id: i64,
    invite_link: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO invite_links (user_id, group_id, invite_link)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, group_id) DO UPDATE SET
             invite_link = excluded.invite_link,
             created_at = CURRENT_TIMESTAMP,
             used_at = NULL,
             revoked_at = NULL",
    )
    .bind(user_id)
    .bind(group_id)
    .bind(invite_link)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_used(pool: &DbPool, invite_link: &str) -> Result<Option<InviteLink>> {
    sqlx::query(
        "UPDATE invite_links SET used_at = CURRENT_TIMESTAMP WHERE invite_link = ? AND used_at IS NULL",
    )
    .bind(invite_link)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, InviteLink>("SELECT * FROM invite_links WHERE invite_link = ?")
        .bind(invite_link)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn revoke_all_unused_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<InviteLink>> {
    let links = sqlx::query_as::<_, InviteLink>(
        "SELECT * FROM invite_links WHERE user_id = ? AND used_at IS NULL AND revoked_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    sqlx::query(
        "UPDATE invite_links SET revoked_at = CURRENT_TIMESTAMP
         WHERE user_id = ? AND used_at IS NULL AND revoked_at IS NULL",
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(links)
}

pub async fn list_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<InviteLink>> {
    sqlx::query_as::<_, InviteLink>(
        "SELECT * FROM invite_links WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn get_by_link(pool: &DbPool, invite_link: &str) -> Result<Option<InviteLink>> {
    sqlx::query_as::<_, InviteLink>("SELECT * FROM invite_links WHERE invite_link = ?")
        .bind(invite_link)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}
