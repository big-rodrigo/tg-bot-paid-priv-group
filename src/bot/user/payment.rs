use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message},
};

use crate::{
    bot::state::{BotDialogue, HandlerResult, State},
    config::AppConfig,
    db::{queries, DbPool},
    error::AppError,
    payment::PaymentProvider,
};

/// Sends the payment method selection keyboard, adapting labels to the configured provider.
pub async fn send_payment_options(
    bot: &Bot,
    chat_id: ChatId,
    payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
) -> HandlerResult {
    let (label, callback) = match payment_provider.provider_name() {
        "livepix" => ("Pay via LivePix", "pay:livepix"),
        "telegram" => ("Pay via Card / Wallet", "pay:telegram"),
        _ => ("Pay via External Gateway", "pay:external"),
    };

    let rows: Vec<Vec<InlineKeyboardButton>> =
        vec![vec![InlineKeyboardButton::callback(label, callback)]];

    let keyboard = InlineKeyboardMarkup::new(rows);
    bot.send_message(
        chat_id,
        "Registration complete! Please proceed with payment to receive your invite links.",
    )
    .reply_markup(keyboard)
    .await?;

    Ok(())
}

/// Reads `livepix_price_cents` from the settings table; returns 0 on error.
async fn read_price_cents(pool: &DbPool) -> i64 {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'livepix_price_cents'")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Handles the payment method selection from the inline keyboard.
pub async fn handle_payment_selection(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    pool: DbPool,
    config: Arc<AppConfig>,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;

    let user_telegram_id = q.from.id.0 as i64;
    let user = queries::users::get_by_telegram_id(&pool, user_telegram_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let chat_id = ChatId(user_telegram_id);

    match q.data.as_deref() {
        Some("pay:livepix") | Some("pay:external") => {
            let provider_key = if q.data.as_deref() == Some("pay:livepix") {
                "livepix"
            } else {
                "external"
            };
            let price_cents = read_price_cents(&pool).await;
            let payment_id =
                queries::payments::create(&pool, user.id, provider_key, Some(price_cents)).await?;
            let initiation = payment_provider.initiate(&user, payment_id).await?;

            if let Some(ext_ref) = &initiation.external_ref {
                queries::payments::set_external_ref(&pool, payment_id, ext_ref).await?;
            }

            bot.send_message(chat_id, &initiation.instructions)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;

            if let Some(url) = &initiation.payment_url {
                bot.send_message(chat_id, url.as_str()).await?;
            }

            dialogue
                .update(State::AwaitingExternalPayment { payment_id })
                .await?;
        }

        Some("pay:telegram") => {
            let provider_token = config
                .telegram_payment_provider_token
                .clone()
                .unwrap_or_default();
            if provider_token.is_empty() {
                bot.send_message(
                    chat_id,
                    "Telegram payments are not configured. Please choose another method.",
                )
                .await?;
                return Ok(());
            }

            let payment_id =
                queries::payments::create(&pool, user.id, "telegram", None).await?;
            let title = "Registration Access";
            let description = "One-time payment for access to private groups";
            let payload = payment_id.to_string();

            bot.send_invoice(
                chat_id,
                title,
                description,
                &payload,
                "USD",
                vec![teloxide::types::LabeledPrice {
                    label: "Access Fee".to_string(),
                    amount: 1000, // $10.00 in cents — configure as needed
                }],
            )
            .provider_token(&provider_token)
            .await?;

            dialogue
                .update(State::AwaitingTelegramPayment { payment_id })
                .await?;
        }

        _ => {
            bot.send_message(chat_id, "Unknown payment option.").await?;
        }
    }

    Ok(())
}

/// Handles `pre_checkout_query` — must be answered within 10 seconds.
pub async fn handle_pre_checkout_query(
    bot: Bot,
    query: teloxide::types::PreCheckoutQuery,
) -> HandlerResult {
    bot.answer_pre_checkout_query(query.id, true).await?;
    Ok(())
}

/// Handles the `successful_payment` update for Telegram payments.
pub async fn handle_successful_payment(
    bot: Bot,
    dialogue: BotDialogue,
    msg: Message,
    pool: DbPool,
) -> HandlerResult {
    let successful_payment = match msg.successful_payment() {
        Some(p) => p,
        None => return Ok(()),
    };

    let payment_id: i64 = successful_payment.invoice_payload.parse().unwrap_or(0);

    queries::payments::complete_telegram(
        &pool,
        payment_id,
        &successful_payment.telegram_payment_charge_id.to_string(),
    )
    .await?;

    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    dialogue.update(State::Registered).await?;

    super::invite::deliver_invites(bot, pool, user.id, user.telegram_id).await?;

    Ok(())
}
