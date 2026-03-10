use std::collections::HashSet;
use std::sync::Arc;

use teloxide::{prelude::*, types::ChatId};
use tokio::sync::RwLock;

use crate::{
    bot::{group::invite_manager, state::HandlerResult, util::escape_html},
    db::{models::Group, queries, DbPool},
    db_query_as, db_query_scalar,
    error::Result,
    i18n::{self, Lang},
    payment::{PaymentProvider, WebhookEvent},
};

/// Creates one-time invite links based on invite phase rules and sends them to the user.
/// Called after payment is confirmed (both external webhook and Telegram payments).
/// Falls back to sending all active groups if no invite phases are configured.
pub async fn deliver_invites(
    bot: Bot,
    pool: DbPool,
    user_id: i64,
    user_telegram_id: i64,
    l: Lang,
) -> Result<()> {
    let chat_id = ChatId(user_telegram_id);

    let invite_phases = queries::phases::list_active_invite(&pool).await?;

    if invite_phases.is_empty() {
        return deliver_all_groups(bot, pool, user_id, user_telegram_id, l).await;
    }

    bot.send_message(chat_id, i18n::payment_confirmed_processing(l))
        .await?;

    let mut total_links = 0;
    let mut invited_group_ids: HashSet<i64> = HashSet::new();

    for phase in &invite_phases {
        // Fetch info blocks and invite rules, process interleaved by position
        let info_blocks = queries::questions::list_by_phase(&pool, phase.id).await?;
        let invite_rules = queries::invite_rules::list_by_phase(&pool, phase.id).await?;

        enum PhaseItem {
            Info {
                text: String,
                position: i64,
                media_path: Option<String>,
                media_type: Option<String>,
                media_file_id: Option<String>,
            },
            Rule { rule_id: i64, group_id: i64, position: i64 },
        }

        let mut items: Vec<PhaseItem> = Vec::new();
        for q in &info_blocks {
            if q.question_type == "info" {
                items.push(PhaseItem::Info {
                    text: q.text.clone(),
                    position: q.position,
                    media_path: q.media_path.clone(),
                    media_type: q.media_type.clone(),
                    media_file_id: q.media_file_id.clone(),
                });
            }
        }
        for rule in &invite_rules {
            items.push(PhaseItem::Rule {
                rule_id: rule.id,
                group_id: rule.group_id,
                position: rule.position,
            });
        }
        items.sort_by_key(|item| match item {
            PhaseItem::Info { position, .. } => *position,
            PhaseItem::Rule { position, .. } => *position,
        });

        for item in &items {
            match item {
                PhaseItem::Info {
                    text,
                    media_path,
                    media_type,
                    media_file_id,
                    ..
                } => {
                    crate::bot::user::media::send_media_or_text(
                        &bot,
                        chat_id,
                        text,
                        media_path.as_deref(),
                        media_type.as_deref(),
                        media_file_id.as_deref(),
                        None,
                    )
                    .await
                    .map_err(|e| crate::error::AppError::Other(e.to_string()))?;
                }
                PhaseItem::Rule {
                    rule_id, group_id, ..
                } => {
                    if invited_group_ids.contains(group_id) {
                        continue;
                    }

                    let matched =
                        queries::invite_rules::evaluate_rule(&pool, *rule_id, user_id).await?;

                    if !matched {
                        continue;
                    }

                    let group = db_query_as!(&pool, Group, "SELECT * FROM groups WHERE id = ? AND active = TRUE", [group_id], fetch_optional)?;

                    let Some(group) = group else { continue };

                    match invite_manager::create_one_time_link(&bot, group.telegram_id).await {
                        Ok(link) => {
                            queries::invite_links::create(&pool, user_id, group.id, &link).await?;
                            bot.send_message(
                                chat_id,
                                i18n::link_line(&escape_html(&group.title), &link),
                            )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                            invited_group_ids.insert(group.id);
                            total_links += 1;
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to create invite link for group {} (id: {}): {e}",
                                group.title,
                                group.telegram_id,
                            );
                            bot.send_message(
                                chat_id,
                                i18n::link_error(l, &escape_html(&group.title)),
                            )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                        }
                    }
                }
            }
        }
    }

    if total_links > 0 {
        bot.send_message(chat_id, i18n::welcome_join(l))
            .await?;
    } else {
        bot.send_message(chat_id, i18n::no_matches(l))
            .await?;
    }

    tracing::info!(
        "Delivered {total_links} invite links to user_id={user_id} (invite phase rules)"
    );

    Ok(())
}

/// Legacy fallback: deliver links for ALL active groups.
/// Used when no invite phases are configured.
async fn deliver_all_groups(
    bot: Bot,
    pool: DbPool,
    user_id: i64,
    user_telegram_id: i64,
    l: Lang,
) -> Result<()> {
    let chat_id = ChatId(user_telegram_id);

    let groups = db_query_as!(&pool, Group, "SELECT * FROM groups WHERE active = TRUE ORDER BY id ASC", [], fetch_all)?;

    if groups.is_empty() {
        bot.send_message(chat_id, i18n::no_groups_configured(l))
            .await?;
        return Ok(());
    }

    bot.send_message(chat_id, i18n::here_are_links(l))
        .await?;

    let mut success_count = 0;

    for group in &groups {
        match invite_manager::create_one_time_link(&bot, group.telegram_id).await {
            Ok(link) => {
                queries::invite_links::create(&pool, user_id, group.id, &link).await?;
                bot.send_message(
                    chat_id,
                    i18n::link_line(&escape_html(&group.title), &link),
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
                    i18n::link_error(l, &escape_html(&group.title)),
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
        }
    }

    if success_count > 0 {
        bot.send_message(chat_id, i18n::welcome_join(l))
            .await?;
    }

    tracing::info!(
        "Delivered {success_count}/{} invite links to user_id={user_id}",
        groups.len()
    );

    Ok(())
}

/// Re-sends existing unused invite links, and generates new ones for groups where all
/// links have been used. Only considers groups the user was previously granted access to.
/// Falls back to all active groups if no invite phases are configured.
pub async fn refresh_invites(
    bot: Bot,
    pool: DbPool,
    user_id: i64,
    user_telegram_id: i64,
    l: Lang,
) -> Result<()> {
    let chat_id = ChatId(user_telegram_id);

    // Find groups the user has been granted access to
    let granted_group_ids: Vec<i64> = db_query_scalar!(&pool, i64, "SELECT DISTINCT group_id FROM invite_links WHERE user_id = ?", [user_id], fetch_all)?;

    // If no granted groups, check for legacy fallback
    let groups: Vec<Group> = if granted_group_ids.is_empty() {
        let invite_phases = queries::phases::list_active_invite(&pool).await?;
        if invite_phases.is_empty() {
            // Legacy: use all active groups
            db_query_as!(&pool, Group, "SELECT * FROM groups WHERE active = TRUE ORDER BY id ASC", [], fetch_all)?
        } else {
            // Invite phases exist but user has no links — nothing to refresh
            bot.send_message(chat_id, i18n::no_links_available(l))
                .await?;
            return Ok(());
        }
    } else {
        // Fetch only the groups the user was granted access to
        let mut result = Vec::new();
        for gid in &granted_group_ids {
            if let Some(group) = db_query_as!(&pool, Group, "SELECT * FROM groups WHERE id = ? AND active = TRUE", [gid], fetch_optional)?
            {
                result.push(group);
            }
        }
        result
    };

    if groups.is_empty() {
        bot.send_message(chat_id, i18n::no_groups_available(l))
            .await?;
        return Ok(());
    }

    let mut sent_any = false;

    for group in &groups {
        let existing = db_query_scalar!(&pool, String, "SELECT invite_link FROM invite_links
             WHERE user_id = ? AND group_id = ?
               AND used_at IS NULL AND revoked_at IS NULL
             ORDER BY created_at DESC
             LIMIT 1", [user_id, group.id], fetch_optional)?;

        let link = if let Some(existing_link) = existing {
            existing_link
        } else {
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
                        i18n::link_error_contact_admin(l, &escape_html(&group.title)),
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
                    continue;
                }
            }
        };

        if !sent_any {
            bot.send_message(chat_id, i18n::links_header(l))
                .await?;
            sent_any = true;
        }

        bot.send_message(
            chat_id,
            i18n::link_line(&escape_html(&group.title), &link),
        )
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    }

    if !sent_any {
        bot.send_message(chat_id, i18n::no_links_available(l))
            .await?;
    }

    Ok(())
}

/// Proactively checks the payment provider's API for a matching payment.
/// If found, marks the payment completed and delivers invites.
/// Returns `true` if a payment was found and processed.
pub async fn try_proactive_payment_check(
    bot: &Bot,
    pool: &DbPool,
    payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
    user: &crate::db::models::User,
    l: Lang,
) -> Result<bool> {
    let pending = queries::payments::get_pending_for_user(pool, user.id).await?;
    let Some(pending) = pending else {
        return Ok(false);
    };

    let event = match payment_provider.check_payment(&pending).await {
        Ok(Some(ev)) => ev,
        Ok(None) => return Ok(false),
        Err(e) => {
            tracing::warn!("Proactive payment check failed for user {}: {e}", user.id);
            return Ok(false);
        }
    };

    if let WebhookEvent::Completed {
        external_ref,
        payload,
        amount,
        currency,
    } = event
    {
        let completed = queries::payments::complete_external(
            pool,
            &external_ref,
            &payload,
            amount,
            currency.as_deref(),
        )
        .await?;

        if completed.is_some() {
            tracing::info!(
                "Proactive payment check: completed payment for user {} (telegram_id={})",
                user.id,
                user.telegram_id
            );
            deliver_invites(bot.clone(), pool.clone(), user.id, user.telegram_id, l).await?;
            return Ok(true);
        }
    }

    Ok(false)
}

/// Handler for the /links command.
pub async fn handle_mylinks(
    bot: Bot,
    msg: Message,
    pool: DbPool,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64).await?;
    let Some(user) = user else {
        bot.send_message(msg.chat.id, i18n::not_registered(l))
            .await?;
        return Ok(());
    };

    let payment = queries::payments::get_completed_for_user(&pool, user.id).await?;
    if payment.is_none() {
        // Try proactive check before giving up
        if try_proactive_payment_check(&bot, &pool, &payment_provider, &user, l).await? {
            return Ok(());
        }
        bot.send_message(msg.chat.id, i18n::no_payment(l))
            .await?;
        return Ok(());
    }

    refresh_invites(bot, pool, user.id, user.telegram_id, l).await?;
    Ok(())
}
