use std::sync::Arc;
use teloxide::{prelude::*, types::{Message, ParseMode}};

use crate::{
    bot::state::{BotDialogue, HandlerResult, State},
    db::{queries, DbPool},
    payment::PaymentProvider,
};

async fn send_welcome_message(bot: &Bot, chat_id: teloxide::types::ChatId, pool: &DbPool) -> HandlerResult {
    let text = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'welcome_message'",
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "Welcome! 👋".to_string());

    bot.send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await?;
    Ok(())
}

pub async fn handle_start(
    bot: Bot,
    dialogue: BotDialogue,
    msg: Message,
    pool: DbPool,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
) -> HandlerResult {
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
            bot.send_message(
                msg.chat.id,
                "You are already registered! Check your previous messages for your invite links.\n\
                 If you need help, contact an administrator.",
            )
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
                send_welcome_message(&bot, msg.chat.id, &pool).await?;
                bot.send_message(
                    msg.chat.id,
                    "Resuming your registration from where you left off.",
                )
                .await?;
                crate::bot::user::registration::send_question(
                    &bot,
                    msg.chat.id,
                    &pool,
                    &question,
                )
                .await?;
                dialogue
                    .update(State::InPhase { phase_id, question_id })
                    .await?;
                return Ok(());
            }
            // Question no longer exists (admin deleted it) — fall through to restart
        }
    }

    // Fresh start: find first active phase
    let phases = queries::phases::list_active(&pool).await?;
    let first_phase = match phases.into_iter().next() {
        Some(p) => p,
        None => {
            bot.send_message(
                msg.chat.id,
                "Registration is not yet configured. Please check back later.",
            )
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

    send_welcome_message(&bot, msg.chat.id, &pool).await?;

    // Start from the first question of the first phase (handles leading info blocks)
    crate::bot::user::registration::start_phase(
        &bot,
        &dialogue,
        &pool,
        &payment_provider,
        msg.chat.id,
        user.id,
        first_phase.id,
    )
    .await
}

pub async fn handle_status(bot: Bot, msg: Message, pool: DbPool) -> HandlerResult {
    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64).await?;
    match user {
        None => {
            bot.send_message(msg.chat.id, "You haven't started registration yet. Send /start to begin.").await?;
        }
        Some(user) => {
            let reg = sqlx::query_as::<_, crate::db::models::UserRegistration>(
                "SELECT * FROM user_registration WHERE user_id = ?",
            )
            .bind(user.id)
            .fetch_optional(&pool)
            .await?;

            let status = match reg {
                None => "Not started",
                Some(r) if r.completed_at.is_some() => "Registered ✅",
                Some(_) => "In progress ⏳",
            };

            bot.send_message(msg.chat.id, format!("Registration status: {status}")).await?;
        }
    }

    Ok(())
}
