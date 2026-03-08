use teloxide::{prelude::*, types::ChatMemberUpdated};

use crate::{
    bot::state::HandlerResult,
    db::{queries, DbPool},
};

/// Handles `ChatMemberUpdated` events sent when someone joins a group.
/// When a user joins via a tracked one-time invite link, the link is marked as used —
/// but only if the joining user is the rightful owner of that link.
/// If someone else uses the link, they are kicked silently.
pub async fn handle(bot: Bot, update: ChatMemberUpdated, pool: DbPool) -> HandlerResult {
    use teloxide::types::ChatMemberStatus;
    if update.new_chat_member.status() != ChatMemberStatus::Member {
        return Ok(());
    }

    // Check if they joined via a tracked invite link
    let invite_link = match update.invite_link.as_ref() {
        Some(link) => link.invite_link.clone(),
        None => return Ok(()),
    };

    let record = match queries::invite_links::get_by_link(&pool, &invite_link).await? {
        Some(r) => r,
        None => return Ok(()),
    };

    if record.used_at.is_some() {
        // Already marked as used (duplicate event)
        return Ok(());
    }

    // Verify the joining user is the link owner
    let joining_telegram_id = update.new_chat_member.user.id.0 as i64;
    let owner = queries::users::get_by_id(&pool, record.user_id).await?;

    if let Some(owner) = owner {
        if owner.telegram_id != joining_telegram_id {
            // Wrong user — kick silently without marking the link as used
            // so the rightful owner can still use /mylinks to regenerate
            let chat_id = update.chat.id;
            let user_id = update.new_chat_member.user.id;
            let _ = bot.ban_chat_member(chat_id, user_id).await;
            let _ = bot.unban_chat_member(chat_id, user_id).await;
            tracing::warn!(
                "Kicked user {} who joined with link belonging to user {} (owner telegram_id={})",
                joining_telegram_id,
                record.user_id,
                owner.telegram_id,
            );
            return Ok(());
        }
    }

    // Correct user — mark the link as used
    queries::invite_links::mark_used(&pool, &invite_link).await?;

    tracing::info!(
        "User {} (DB id: {}) joined group via invite link",
        update.new_chat_member.user.id,
        record.user_id,
    );

    Ok(())
}
