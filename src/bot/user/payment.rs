use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup},
};
use tokio::sync::RwLock;

use crate::{
    bot::state::{BotDialogue, HandlerResult, State},
    db::{queries, DbPool},
    db_query_scalar,
    error::AppError,
    i18n::{self, Lang},
    payment::PaymentProvider,
};

/// Sends the payment method selection keyboard.
pub async fn send_payment_options(
    bot: &Bot,
    chat_id: ChatId,
    _payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
    l: Lang,
) -> HandlerResult {
    let label = i18n::pay_livepix(l);
    let callback = "pay:livepix";

    let rows: Vec<Vec<InlineKeyboardButton>> =
        vec![vec![InlineKeyboardButton::callback(label, callback)]];

    let keyboard = InlineKeyboardMarkup::new(rows);
    bot.send_message(chat_id, i18n::registration_complete(l))
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

/// Reads `livepix_price_cents` from the settings table; returns 0 on error.
async fn read_price_cents(pool: &DbPool) -> i64 {
    db_query_scalar!(pool, String, "SELECT value FROM settings WHERE key = 'livepix_price_cents'", [], fetch_optional)
        .ok()
        .flatten()
        .and_then(|v: String| v.parse().ok())
        .unwrap_or(0)
}

/// Handles the payment method selection from the inline keyboard.
pub async fn handle_payment_selection(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    pool: DbPool,
    _config: Arc<crate::config::AppConfig>,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    bot.answer_callback_query(q.id).await?;

    let user_telegram_id = q.from.id.0 as i64;
    let user = queries::users::get_by_telegram_id(&pool, user_telegram_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let chat_id = ChatId(user_telegram_id);

    match q.data.as_deref() {
        Some("pay:livepix") => {
            let price_cents = read_price_cents(&pool).await;
            let payment_id =
                queries::payments::create(&pool, user.id, "livepix", Some(price_cents)).await?;
            let initiation = payment_provider.initiate(&user, payment_id).await?;

            if let Some(ext_ref) = &initiation.external_ref {
                queries::payments::set_external_ref(&pool, payment_id, ext_ref).await?;
            }

            bot.send_message(chat_id, &initiation.instructions)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;

            if let Some(url) = &initiation.payment_url {
                let link_msg = format!("<a href=\"{url}\">{url}</a>");
                bot.send_message(chat_id, &link_msg)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }

            dialogue
                .update(State::AwaitingPaymentConfirmation { payment_id })
                .await?;
        }

        _ => {
            bot.send_message(chat_id, i18n::unknown_payment_option(l))
                .await?;
        }
    }

    Ok(())
}
