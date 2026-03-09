use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::RwLock;

use crate::{
    bot::state::{BotDialogue, HandlerResult, State},
    db::{queries, DbPool},
    i18n::{self, Lang},
    payment::PaymentProvider,
};

pub async fn handle_start(
    bot: Bot,
    dialogue: BotDialogue,
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

    // Upsert the user in the database
    let user = queries::users::upsert(
        &pool,
        from.id.0 as i64,
        from.username.as_deref(),
        &from.first_name,
        from.last_name.as_deref(),
    )
    .await?;

    // Check existing registration
    let reg = sqlx::query_as::<_, crate::db::models::UserRegistration>(
        "SELECT * FROM user_registration WHERE user_id = ?",
    )
    .bind(user.id)
    .fetch_optional(&pool)
    .await?;

    if let Some(ref r) = reg {
        // Already fully registered
        if r.completed_at.is_some() {
            bot.send_message(msg.chat.id, i18n::already_registered(l))
                .await?;
            dialogue.update(State::Registered).await?;
            return Ok(());
        }

        // Registration in progress — resume from where they left off
        if let (Some(phase_id), Some(question_id)) =
            (r.current_phase_id, r.current_question_id)
        {
            let question = queries::questions::get_by_id(&pool, question_id).await?;
            if let Some(question) = question {
                bot.send_message(msg.chat.id, i18n::resuming_registration(l))
                    .await?;
                crate::bot::user::registration::send_question(
                    &bot,
                    msg.chat.id,
                    &pool,
                    &question,
                    l,
                )
                .await?;
                dialogue
                    .update(State::InPhase { phase_id, question_id })
                    .await?;
                return Ok(());
            }
            // Question no longer exists (admin deleted it) — fall through to restart
        }

        // Registration exists but not completed and can't resume a question.
        // Check for pending payment before resetting registration.
        if super::invite::try_proactive_payment_check(&bot, &pool, &payment_provider, &user, l)
            .await?
        {
            dialogue.update(State::Registered).await?;
            return Ok(());
        }
    }

    // Fresh start: find first active phase
    let phases = queries::phases::list_active_normal(&pool).await?;
    let first_phase = match phases.into_iter().next() {
        Some(p) => p,
        None => {
            bot.send_message(msg.chat.id, i18n::not_configured(l))
                .await?;
            return Ok(());
        }
    };

    // Insert / reset user_registration row
    sqlx::query(
        "INSERT INTO user_registration (user_id, current_phase_id, current_question_id)
         VALUES (?, ?, NULL)
         ON CONFLICT(user_id) DO UPDATE SET
             current_phase_id = excluded.current_phase_id,
             current_question_id = NULL,
             completed_at = NULL",
    )
    .bind(user.id)
    .bind(first_phase.id)
    .execute(&pool)
    .await?;

    // Start from the first question of the first phase (handles leading info blocks)
    crate::bot::user::registration::start_phase(
        &bot,
        &dialogue,
        &pool,
        &payment_provider,
        msg.chat.id,
        user.id,
        first_phase.id,
        l,
    )
    .await
}

pub async fn handle_status(
    bot: Bot,
    msg: Message,
    pool: DbPool,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64).await?;
    match user {
        None => {
            bot.send_message(msg.chat.id, i18n::not_started(l)).await?;
        }
        Some(user) => {
            let reg = sqlx::query_as::<_, crate::db::models::UserRegistration>(
                "SELECT * FROM user_registration WHERE user_id = ?",
            )
            .bind(user.id)
            .fetch_optional(&pool)
            .await?;

            let status = match reg {
                None => i18n::status_not_started(l),
                Some(r) if r.completed_at.is_some() => i18n::status_registered(l),
                Some(_) => i18n::status_in_progress(l),
            };

            bot.send_message(msg.chat.id, i18n::status_format(l, status))
                .await?;
        }
    }

    Ok(())
}
