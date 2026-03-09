use std::sync::Arc;
use teloxide::{
    prelude::*,
    types::{ChatId, FileId, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, Message, ParseMode},
};
use tokio::sync::RwLock;

use crate::{
    bot::state::{BotDialogue, HandlerResult, State},
    db::{models::Question, queries, DbPool},
    db_execute,
    error::AppError,
    i18n::{self, Lang},
    payment::PaymentProvider,
};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Extracts the Telegram file_id from a sent message based on media type.
fn extract_file_id(msg: &Message, media_type: &str) -> Option<String> {
    match media_type {
        "image" => msg.photo().and_then(|photos| {
            photos.iter().max_by_key(|p| p.width * p.height).map(|p| p.file.id.to_string())
        }),
        "video" => msg.video().map(|v| v.file.id.to_string()),
        "animation" => msg.animation().map(|a| a.file.id.to_string()),
        _ => msg.document().map(|d| d.file.id.to_string()),
    }
}

/// Sends a message that may include a media attachment (image/video/animation).
/// If media is present, uses the appropriate Telegram media method with caption.
/// Falls back to a plain text message when no media is attached.
/// Telegram caption limit is 1024 chars — if text exceeds that, media is sent
/// without caption first, then text as a separate message.
///
/// If `media_file_id` is provided, uses Telegram's cached file instead of re-uploading.
/// Returns the Telegram file_id of the sent media (if any) for caching.
pub(crate) async fn send_media_or_text(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    media_path: Option<&str>,
    media_type: Option<&str>,
    media_file_id: Option<&str>,
    reply_markup: Option<InlineKeyboardMarkup>,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    match (media_path, media_type) {
        (Some(path), Some(mtype)) => {
            let input_file = if let Some(fid) = media_file_id {
                InputFile::file_id(FileId(fid.to_owned()))
            } else {
                InputFile::file(std::path::Path::new(path))
            };
            let text_fits_caption = text.len() <= 1024;

            let sent_msg = match mtype {
                "image" => {
                    let mut req = bot.send_photo(chat_id, input_file);
                    if text_fits_caption {
                        req = req.caption(text).parse_mode(ParseMode::Html);
                    }
                    if text_fits_caption {
                        if let Some(kb) = &reply_markup {
                            req = req.reply_markup(kb.clone());
                        }
                    }
                    req.await?
                }
                "video" => {
                    let mut req = bot.send_video(chat_id, input_file)
                        .supports_streaming(true);
                    if text_fits_caption {
                        req = req.caption(text).parse_mode(ParseMode::Html);
                    }
                    if text_fits_caption {
                        if let Some(kb) = &reply_markup {
                            req = req.reply_markup(kb.clone());
                        }
                    }
                    req.await?
                }
                "animation" => {
                    let mut req = bot.send_animation(chat_id, input_file);
                    if text_fits_caption {
                        req = req.caption(text).parse_mode(ParseMode::Html);
                    }
                    if text_fits_caption {
                        if let Some(kb) = &reply_markup {
                            req = req.reply_markup(kb.clone());
                        }
                    }
                    req.await?
                }
                _ => {
                    // Unknown media type — send as document with caption
                    let mut req = bot.send_document(chat_id, input_file);
                    if text_fits_caption {
                        req = req.caption(text).parse_mode(ParseMode::Html);
                    }
                    if text_fits_caption {
                        if let Some(kb) = &reply_markup {
                            req = req.reply_markup(kb.clone());
                        }
                    }
                    req.await?
                }
            };

            // If text was too long for caption, send it as a separate message
            if !text_fits_caption {
                let mut msg_req = bot.send_message(chat_id, text).parse_mode(ParseMode::Html);
                if let Some(kb) = reply_markup {
                    msg_req = msg_req.reply_markup(kb);
                }
                msg_req.await?;
            }

            // Return file_id from sent message (only useful when we uploaded fresh)
            if media_file_id.is_none() {
                Ok(extract_file_id(&sent_msg, mtype))
            } else {
                Ok(None)
            }
        }
        _ => {
            // No media — plain text message
            let mut req = bot.send_message(chat_id, text).parse_mode(ParseMode::Html);
            if let Some(kb) = reply_markup {
                req = req.reply_markup(kb);
            }
            req.await?;
            Ok(None)
        }
    }
}

/// Helper: sends media via `send_media_or_text` and caches the file_id if it was a fresh upload.
async fn send_and_cache_file_id(
    bot: &Bot,
    chat_id: ChatId,
    pool: &DbPool,
    question_id: i64,
    text: &str,
    media_path: Option<&str>,
    media_type: Option<&str>,
    media_file_id: Option<&str>,
    reply_markup: Option<InlineKeyboardMarkup>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let new_file_id = send_media_or_text(
        bot, chat_id, text, media_path, media_type, media_file_id, reply_markup,
    )
    .await?;
    if let Some(fid) = new_file_id {
        let _ = queries::questions::update_media_file_id(pool, question_id, &fid).await;
    }
    Ok(())
}

/// Sends a question to the user, building the appropriate keyboard for button-type questions.
pub async fn send_question(
    bot: &Bot,
    chat_id: ChatId,
    pool: &DbPool,
    question: &Question,
    l: Lang,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let media_path = question.media_path.as_deref();
    let media_type = question.media_type.as_deref();
    let media_file_id = question.media_file_id.as_deref();

    match question.question_type.as_str() {
        "button" => {
            let options = queries::questions::list_options(pool, question.id).await?;
            if options.is_empty() {
                send_and_cache_file_id(
                    bot, chat_id, pool, question.id,
                    &question.text, media_path, media_type, media_file_id, None,
                ).await?;
                return Ok(());
            }
            let buttons: Vec<Vec<InlineKeyboardButton>> = options
                .iter()
                .map(|opt| {
                    vec![InlineKeyboardButton::callback(
                        opt.label.clone(),
                        format!("q{}:opt:{}", question.id, opt.id),
                    )]
                })
                .collect();
            let keyboard = InlineKeyboardMarkup::new(buttons);
            send_and_cache_file_id(
                bot, chat_id, pool, question.id,
                &question.text, media_path, media_type, media_file_id, Some(keyboard),
            ).await?;
        }
        "image" => {
            let text = format!("{}\n\n{}", question.text, i18n::photo_prompt(l));
            send_and_cache_file_id(
                bot, chat_id, pool, question.id,
                &text, media_path, media_type, media_file_id, None,
            ).await?;
        }
        _ => {
            // text, info, or unknown — just send the text
            send_and_cache_file_id(
                bot, chat_id, pool, question.id,
                &question.text, media_path, media_type, media_file_id, None,
            ).await?;
        }
    }
    Ok(())
}

/// Starts registration from the first question of the given phase, auto-advancing
/// through any leading info blocks until the first interactive question.
pub async fn start_phase(
    bot: &Bot,
    dialogue: &BotDialogue,
    pool: &DbPool,
    payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
    chat_id: ChatId,
    user_id: i64,
    phase_id: i64,
    l: Lang,
) -> HandlerResult {
    let first_q = queries::questions::first_in_phase(pool, phase_id).await?;
    match first_q {
        None => {
            all_phases_complete(bot, dialogue, pool, payment_provider, chat_id, user_id, l).await
        }
        Some(q) => {
            db_execute!(pool, "UPDATE user_registration SET current_phase_id = ?, current_question_id = ? WHERE user_id = ?", [phase_id, q.id, user_id])?;

            if q.question_type == "info" {
                send_and_cache_file_id(
                    bot, chat_id, pool, q.id,
                    &q.text, q.media_path.as_deref(), q.media_type.as_deref(),
                    q.media_file_id.as_deref(), None,
                )
                .await?;
                advance(bot, dialogue, pool, payment_provider, chat_id, user_id, phase_id, &q, l)
                    .await
            } else {
                send_question(bot, chat_id, pool, &q, l).await?;
                dialogue
                    .update(State::InPhase {
                        phase_id,
                        question_id: q.id,
                    })
                    .await?;
                Ok(())
            }
        }
    }
}

/// Handles incoming messages (text or image) when the user is in a phase.
pub async fn handle_message(
    bot: Bot,
    dialogue: BotDialogue,
    msg: Message,
    pool: DbPool,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    let state = dialogue.get().await?.unwrap_or_default();
    let (phase_id, question_id) = match state {
        State::InPhase { phase_id, question_id } => (phase_id, question_id),
        _ => return Ok(()),
    };

    let from = match msg.from.as_ref() {
        Some(u) => u,
        None => return Ok(()),
    };

    let user = queries::users::get_by_telegram_id(&pool, from.id.0 as i64)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    let question = queries::questions::get_by_id(&pool, question_id)
        .await?
        .ok_or_else(|| AppError::NotFound("question not found".into()))?;

    match question.question_type.as_str() {
        "text" => {
            if let Some(text) = msg.text() {
                queries::answers::save_text(&pool, user.id, question_id, text).await?;
                advance(
                    &bot,
                    &dialogue,
                    &pool,
                    &payment_provider,
                    msg.chat.id,
                    user.id,
                    phase_id,
                    &question,
                    l,
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, i18n::send_text(l))
                    .await?;
            }
        }
        "image" => {
            if let Some(photos) = msg.photo() {
                let file_id = photos
                    .iter()
                    .max_by_key(|p| p.width * p.height)
                    .map(|p| p.file.id.to_string())
                    .unwrap_or_default();
                queries::answers::save_image(&pool, user.id, question_id, &file_id).await?;
                advance(
                    &bot,
                    &dialogue,
                    &pool,
                    &payment_provider,
                    msg.chat.id,
                    user.id,
                    phase_id,
                    &question,
                    l,
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, i18n::send_photo(l))
                    .await?;
            }
        }
        "button" => {
            bot.send_message(msg.chat.id, i18n::use_buttons(l))
                .await?;
            send_question(&bot, msg.chat.id, &pool, &question, l).await?;
        }
        "info" => {
            advance(
                &bot,
                &dialogue,
                &pool,
                &payment_provider,
                msg.chat.id,
                user.id,
                phase_id,
                &question,
                l,
            )
            .await?;
        }
        _ => {}
    }

    Ok(())
}

/// Handles inline keyboard callback queries when the user is in a phase.
pub async fn handle_callback(
    bot: Bot,
    dialogue: BotDialogue,
    q: CallbackQuery,
    pool: DbPool,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    let state = dialogue.get().await?.unwrap_or_default();
    let (phase_id, question_id) = match state {
        State::InPhase { phase_id, question_id } => (phase_id, question_id),
        _ => {
            bot.answer_callback_query(q.id).await?;
            return Ok(());
        }
    };

    bot.answer_callback_query(q.id).await?;

    let data = match q.data.as_deref() {
        Some(d) => d.to_owned(),
        None => return Ok(()),
    };

    // Callback data format: "q<question_id>:opt:<option_id>"
    // Also accepts legacy "opt:<option_id>" for backwards compat with in-flight messages
    let parsed = if let Some(rest) = data.strip_prefix("q") {
        // New format: "q<qid>:opt:<oid>"
        rest.split_once(":opt:").and_then(|(qid_str, oid_str)| {
            let qid = qid_str.parse::<i64>().ok()?;
            let oid = oid_str.parse::<i64>().ok()?;
            Some((qid, oid))
        })
    } else if let Some(oid_str) = data.strip_prefix("opt:") {
        // Legacy format: "opt:<oid>" — assume current question
        oid_str.parse::<i64>().ok().map(|oid| (question_id, oid))
    } else {
        None
    };

    if let Some((callback_question_id, option_id)) = parsed {
        // Ignore stale callbacks from a previous question
        if callback_question_id != question_id {
            return Ok(());
        }

        let user_telegram_id = q.from.id.0 as i64;
        let user = queries::users::get_by_telegram_id(&pool, user_telegram_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        let question = queries::questions::get_by_id(&pool, question_id)
            .await?
            .ok_or_else(|| AppError::NotFound("question not found".into()))?;

        queries::answers::save_option(&pool, user.id, question_id, option_id).await?;

        // Edit the original message to show selected option and remove keyboard
        if let Some(msg) = q.message {
            if let Some(msg) = msg.regular_message() {
                let option_label = queries::questions::get_option_by_id(&pool, option_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|o| o.label);
                if let Some(label) = option_label {
                    let original_text = msg.text().or(msg.caption()).unwrap_or(&question.text);
                    let new_text = format!("{}\n\n✓ {}", original_text, label);
                    let _ = bot
                        .edit_message_text(msg.chat.id, msg.id, new_text)
                        .parse_mode(ParseMode::Html)
                        .await;
                } else {
                    let _ = bot
                        .edit_message_reply_markup(msg.chat.id, msg.id)
                        .await;
                }
            }
        }

        let chat_id = ChatId(user_telegram_id);
        advance(
            &bot,
            &dialogue,
            &pool,
            &payment_provider,
            chat_id,
            user.id,
            phase_id,
            &question,
            l,
        )
        .await?;
    }

    Ok(())
}

/// Advances the dialogue to the next question, automatically skipping info blocks
/// (which are sent immediately without waiting for user input).
/// Uses a loop to avoid recursion for consecutive info blocks.
pub(crate) async fn advance(
    bot: &Bot,
    dialogue: &BotDialogue,
    pool: &DbPool,
    payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
    chat_id: ChatId,
    user_id: i64,
    start_phase_id: i64,
    current_question: &Question,
    l: Lang,
) -> HandlerResult {
    let mut phase_id = start_phase_id;
    let mut after_pos = current_question.position;

    loop {
        // Look for the next question in the current phase
        if let Some(next_q) =
            queries::questions::next_in_phase(pool, phase_id, after_pos).await?
        {
            db_execute!(pool, "UPDATE user_registration SET current_phase_id = ?, current_question_id = ? WHERE user_id = ?", [phase_id, next_q.id, user_id])?;

            if next_q.question_type == "info" {
                send_and_cache_file_id(
                    bot, chat_id, pool, next_q.id,
                    &next_q.text, next_q.media_path.as_deref(), next_q.media_type.as_deref(),
                    next_q.media_file_id.as_deref(), None,
                )
                .await?;
                after_pos = next_q.position;
                continue;
            }

            send_question(bot, chat_id, pool, &next_q, l).await?;
            dialogue
                .update(State::InPhase {
                    phase_id,
                    question_id: next_q.id,
                })
                .await?;
            return Ok(());
        }

        // No more questions in this phase — find the next active phase
        let current_phase = queries::phases::get_by_id(pool, phase_id)
            .await?
            .ok_or_else(|| AppError::NotFound("phase not found".into()))?;

        let phases = queries::phases::list_active_normal(pool).await?;
        match phases.into_iter().find(|p| p.position > current_phase.position) {
            Some(next_phase) => {
                bot.send_message(
                    chat_id,
                    i18n::moving_on_to(l, &escape_html(&next_phase.name)),
                )
                .parse_mode(ParseMode::Html)
                .await?;
                phase_id = next_phase.id;
                after_pos = -1;
            }
            None => {
                return all_phases_complete(
                    bot,
                    dialogue,
                    pool,
                    payment_provider,
                    chat_id,
                    user_id,
                    l,
                )
                .await;
            }
        }
    }
}

async fn all_phases_complete(
    bot: &Bot,
    dialogue: &BotDialogue,
    pool: &DbPool,
    payment_provider: &Arc<dyn PaymentProvider + Send + Sync>,
    chat_id: ChatId,
    user_id: i64,
    l: Lang,
) -> HandlerResult {
    db_execute!(pool, "UPDATE user_registration SET completed_at = CURRENT_TIMESTAMP WHERE user_id = ?", [user_id])?;

    dialogue.update(State::AwaitingPayment).await?;

    crate::bot::user::payment::send_payment_options(bot, chat_id, payment_provider, l).await
}
