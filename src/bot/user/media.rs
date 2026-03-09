use teloxide::{
    prelude::*,
    types::{ChatId, FileId, InlineKeyboardMarkup, InputFile, Message, ParseMode},
};

use crate::db::{queries, DbPool};

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
pub async fn send_media_or_text(
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
pub async fn send_and_cache_file_id(
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
