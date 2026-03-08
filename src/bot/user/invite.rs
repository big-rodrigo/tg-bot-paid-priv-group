use teloxide::{prelude::*, types::ChatId};

use crate::{
    bot::{group::invite_manager, state::HandlerResult},
    db::{models::Group, queries, DbPool},
    error::Result,
};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Creates one-time invite links for all active groups and sends them to the user.
/// Called after payment is confirmed (both external webhook and Telegram payments).
pub async fn deliver_invites(
    bot: Bot,
    pool: DbPool,
    user_id: i64,
    user_telegram_id: i64,
) -> Result<()> {
    let chat_id = ChatId(user_telegram_id);

    let groups = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE active = TRUE ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await?;

    if groups.is_empty() {
        bot.send_message(
            chat_id,
            "Payment confirmed! No groups are configured yet — an admin will send your links shortly.",
        )
        .await?;
        return Ok(());
    }

    bot.send_message(
        chat_id,
        "✅ Payment confirmed! Here are your personal invite links:\n\
         ⚠️ Each link can only be used once.",
    )
    .await?;

    let mut success_count = 0;

    for group in &groups {
        match invite_manager::create_one_time_link(&bot, group.telegram_id).await {
            Ok(link) => {
                queries::invite_links::create(&pool, user_id, group.id, &link).await?;
                bot.send_message(
                    chat_id,
                    format!("🔗 <b>{}</b>\n{}", escape_html(&group.title), link),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
                success_count += 1;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create invite link for group {} (id: {}): {e}",
                    group.title,
                    group.telegram_id,
                );
                bot.send_message(
                    chat_id,
                    format!(
                        "⚠️ Could not generate link for <b>{}</b>. An admin will contact you.",
                        escape_html(&group.title)
                    ),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
        }
    }

    if success_count > 0 {
        bot.send_message(
            chat_id,
            "🎉 Welcome! Join the groups using the links above.\n\
             Remember: each link is single-use, so join promptly!",
        )
        .await?;
    }

    tracing::info!(
        "Delivered {success_count}/{} invite links to user_id={user_id}",
        groups.len()
    );

    Ok(())
}

/// Re-sends existing unused invite links, and generates new ones for groups where all
/// links have been used. Called by the /mylinks command.
pub async fn refresh_invites(
    bot: Bot,
    pool: DbPool,
    user_id: i64,
    user_telegram_id: i64,
) -> Result<()> {
    let chat_id = ChatId(user_telegram_id);

    let groups = sqlx::query_as::<_, Group>(
        "SELECT * FROM groups WHERE active = TRUE ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await?;

    if groups.is_empty() {
        bot.send_message(chat_id, "No groups are configured yet. Contact an admin.")
            .await?;
        return Ok(());
    }

    let mut sent_any = false;

    for group in &groups {
        // Check for an existing unused, unrevoked link for this user+group
        let existing = sqlx::query_scalar::<_, String>(
            "SELECT invite_link FROM invite_links
             WHERE user_id = ? AND group_id = ?
               AND used_at IS NULL AND revoked_at IS NULL
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(user_id)
        .bind(group.id)
        .fetch_optional(&pool)
        .await?;

        let link = if let Some(existing_link) = existing {
            existing_link
        } else {
            // All previous links for this group are used/revoked — generate a new one
            match invite_manager::create_one_time_link(&bot, group.telegram_id).await {
                Ok(new_link) => {
                    queries::invite_links::create(&pool, user_id, group.id, &new_link).await?;
                    new_link
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create invite link for group {} (id: {}): {e}",
                        group.title,
                        group.telegram_id,
                    );
                    bot.send_message(
                        chat_id,
                        format!(
                            "⚠️ Could not generate link for <b>{}</b>. Contact an admin.",
                            escape_html(&group.title)
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                    continue;
                }
            }
        };

        if !sent_any {
            bot.send_message(
                chat_id,
                "Here are your invite links:\n⚠️ Each link can only be used once.",
            )
            .await?;
            sent_any = true;
        }

        bot.send_message(
            chat_id,
            format!("🔗 <b>{}</b>\n{}", escape_html(&group.title), link),
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }

    if !sent_any {
        bot.send_message(chat_id, "You have no invite links available. Contact an admin.")
            .await?;
    }

    Ok(())
}

/// Handler for the /mylinks command.
pub async fn handle_mylinks(bot: Bot, msg: Message, pool: DbPool) -> HandlerResult {
    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64).await?;
    let Some(user) = user else {
        bot.send_message(
            msg.chat.id,
            "You haven't registered yet. Send /start to begin.",
        )
        .await?;
        return Ok(());
    };

    let payment = queries::payments::get_completed_for_user(&pool, user.id).await?;
    if payment.is_none() {
        bot.send_message(
            msg.chat.id,
            "You don't have a completed payment yet. Complete registration and payment first.",
        )
        .await?;
        return Ok(());
    }

    refresh_invites(bot, pool, user.id, user.telegram_id).await?;
    Ok(())
}
