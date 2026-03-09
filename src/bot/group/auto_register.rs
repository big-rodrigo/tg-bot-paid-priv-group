use std::sync::Arc;

use teloxide::{
    prelude::*,
    types::{ChatMemberStatus, ChatMemberUpdated},
};

use crate::{bot::state::HandlerResult, config::AppConfig, db::DbPool, db_execute};

/// Handles `MyChatMember` updates — fired when the bot itself is added to or removed from a chat.
/// When added by the configured admin, the group is automatically registered.
/// When removed, the group is marked inactive.
pub async fn handle(
    bot: Bot,
    update: ChatMemberUpdated,
    pool: DbPool,
    config: Arc<AppConfig>,
) -> HandlerResult {
    // Only handle group/supergroup/channel chats
    use teloxide::types::ChatKind;
    if matches!(update.chat.kind, ChatKind::Private(_)) {
        return Ok(());
    }

    let new_status = update.new_chat_member.status();
    let old_status = update.old_chat_member.status();

    let joined = matches!(
        new_status,
        ChatMemberStatus::Member | ChatMemberStatus::Administrator
    ) && matches!(
        old_status,
        ChatMemberStatus::Left | ChatMemberStatus::Banned | ChatMemberStatus::Restricted
    );

    let left = matches!(
        new_status,
        ChatMemberStatus::Left | ChatMemberStatus::Banned
    ) && matches!(
        old_status,
        ChatMemberStatus::Member | ChatMemberStatus::Administrator
    );

    if joined {
        // Only register groups where the admin added the bot
        let from_username = update.from.username.as_deref().unwrap_or("");
        if !from_username.eq_ignore_ascii_case(&config.admin_telegram_username) {
            tracing::info!(
                "Bot was added to group '{}' by non-admin '{}' — leaving",
                update.chat.title().unwrap_or("unknown"),
                from_username
            );
            let _ = bot.leave_chat(update.chat.id).await;
            return Ok(());
        }

        let telegram_id = update.chat.id.0;
        let title = update.chat.title().unwrap_or("Unknown Group");

        db_execute!(&pool, "INSERT INTO groups (telegram_id, title, active)
             VALUES (?, ?, TRUE)
             ON CONFLICT(telegram_id) DO UPDATE SET title = excluded.title, active = TRUE", [telegram_id, title])?;

        tracing::info!("Auto-registered group '{}' (telegram_id: {})", title, telegram_id);
    } else if left {
        let telegram_id = update.chat.id.0;
        db_execute!(&pool, "UPDATE groups SET active = FALSE WHERE telegram_id = ?", [telegram_id])?;

        tracing::info!(
            "Bot left/was removed from group (telegram_id: {}), marked inactive",
            telegram_id
        );
    }

    Ok(())
}
