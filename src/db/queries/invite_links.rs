use crate::{
    db::{models::InviteLink, DbPool},
    db_execute, db_query_as,
    error::Result,
};

pub async fn create(
    pool: &DbPool,
    user_id: i64,
    group_id: i64,
    invite_link: &str,
) -> Result<()> {
    db_execute!(pool,
        "INSERT INTO invite_links (user_id, group_id, invite_link)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, group_id) DO UPDATE SET
             invite_link = excluded.invite_link,
             created_at = CURRENT_TIMESTAMP,
             used_at = NULL,
             revoked_at = NULL",
        [user_id, group_id, invite_link])?;
    Ok(())
}

pub async fn mark_used(pool: &DbPool, invite_link: &str) -> Result<Option<InviteLink>> {
    db_execute!(pool,
        "UPDATE invite_links SET used_at = CURRENT_TIMESTAMP WHERE invite_link = ? AND used_at IS NULL",
        [invite_link])?;

    db_query_as!(pool, InviteLink, "SELECT * FROM invite_links WHERE invite_link = ?", [invite_link], fetch_optional)
        .map_err(Into::into)
}

pub async fn revoke_all_unused_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<InviteLink>> {
    let links = db_query_as!(pool, InviteLink,
        "SELECT * FROM invite_links WHERE user_id = ? AND used_at IS NULL AND revoked_at IS NULL",
        [user_id], fetch_all)?;

    db_execute!(pool,
        "UPDATE invite_links SET revoked_at = CURRENT_TIMESTAMP
         WHERE user_id = ? AND used_at IS NULL AND revoked_at IS NULL",
        [user_id])?;

    Ok(links)
}

pub async fn list_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<InviteLink>> {
    db_query_as!(pool, InviteLink, "SELECT * FROM invite_links WHERE user_id = ? ORDER BY created_at DESC", [user_id], fetch_all)
        .map_err(Into::into)
}

pub async fn get_by_link(pool: &DbPool, invite_link: &str) -> Result<Option<InviteLink>> {
    db_query_as!(pool, InviteLink, "SELECT * FROM invite_links WHERE invite_link = ?", [invite_link], fetch_optional)
        .map_err(Into::into)
}
