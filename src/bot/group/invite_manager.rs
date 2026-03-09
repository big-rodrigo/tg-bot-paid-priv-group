use teloxide::{prelude::*, types::ChatId};

use crate::{db::DbPool, db_query_as, error::Result};

/// Create a one-time invite link for a group.
/// The link can only be used once (`member_limit = 1`).
pub async fn create_one_time_link(bot: &Bot, group_telegram_id: i64) -> Result<String> {
    let link = bot
        .create_chat_invite_link(ChatId(group_telegram_id))
        .member_limit(1u32)
        .await?;

    Ok(link.invite_link)
}

/// Revoke a chat invite link so it can no longer be used.
pub async fn revoke_link(bot: &Bot, group_telegram_id: i64, invite_link: &str) -> Result<()> {
    bot.revoke_chat_invite_link(ChatId(group_telegram_id), invite_link)
        .await?;
    Ok(())
}

/// Revoke all unused invite links for a user across all active groups.
pub async fn revoke_unused_for_user(bot: &Bot, pool: &DbPool, user_id: i64) -> Result<()> {
    use crate::db::{models::Group, queries};

    let links = queries::invite_links::revoke_all_unused_for_user(pool, user_id).await?;

    for link in &links {
        let group = db_query_as!(pool, Group, "SELECT * FROM groups WHERE id = ?", [link.group_id], fetch_optional)?;

        if let Some(group) = group {
            if let Err(e) = revoke_link(bot, group.telegram_id, &link.invite_link).await {
                // Log but don't fail — the link may already be expired or revoked on Telegram's side
                tracing::warn!(
                    "Failed to revoke link {} for user {}: {e}",
                    link.invite_link,
                    user_id
                );
            }
        }
    }

    Ok(())
}
