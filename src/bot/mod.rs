pub mod admin;
pub mod commands;
pub mod group;
pub mod state;
pub mod user;
pub mod util;

use std::sync::Arc;
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::RwLock;

use crate::{
    config::AppConfig,
    db::DbPool,
    i18n::{self, Lang},
    payment::PaymentProvider,
};

use commands::{AdminCommand, UserCommand};
use state::{BotStorage, HandlerResult, State};

/// Build and run the Teloxide dispatcher. This function runs indefinitely.
pub async fn run_dispatcher(
    bot: Bot,
    storage: Arc<BotStorage>,
    pool: DbPool,
    config: Arc<AppConfig>,
    payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    lang: Arc<RwLock<Lang>>,
) {
    // Register user-visible commands with Telegram so they appear in the "/" menu
    if let Err(e) = bot.set_my_commands(UserCommand::bot_commands()).await {
        tracing::warn!("Failed to set bot commands: {e}");
    }

    Dispatcher::builder(bot.clone(), build_handler())
        .dependencies(dptree::deps![storage, pool, config, payment_provider, lang])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn build_handler() -> teloxide::dispatching::UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {

    // Branch for message updates (text, photos, commands)
    let message_handler = Update::filter_message()
        .enter_dialogue::<Message, BotStorage, State>()
        // Admin commands — checked first so admins bypass regular flow
        .branch(
            dptree::filter(admin::guards::is_admin)
                .filter_command::<AdminCommand>()
                .endpoint(admin::commands::handle),
        )
        // /start — works from any state
        .branch(
            dptree::filter_map(|msg: Message| {
                msg.text().and_then(|t| {
                    let trimmed = t.trim();
                    if trimmed == "/start" || trimmed.starts_with("/start ") {
                        Some(())
                    } else {
                        None
                    }
                })
            })
            .endpoint(user::welcome::handle_start),
        )
        // /help
        .branch(
            dptree::filter_map(|msg: Message| {
                msg.text()
                    .and_then(|t| if t.trim() == "/help" { Some(()) } else { None })
            })
            .endpoint(handle_help),
        )
        // /status
        .branch(
            dptree::filter_map(|msg: Message| {
                msg.text()
                    .and_then(|t| if t.trim() == "/status" { Some(()) } else { None })
            })
            .endpoint(user::welcome::handle_status),
        )
        // /mylinks
        .branch(
            dptree::filter_map(|msg: Message| {
                msg.text()
                    .and_then(|t| if t.trim() == "/mylinks" { Some(()) } else { None })
            })
            .endpoint(user::invite::handle_mylinks),
        )
        // In-phase message handler (text / image answers)
        .branch(
            dptree::case![State::InPhase { phase_id, question_id }]
                .endpoint(user::registration::handle_message),
        );

    // Branch for callback queries (button answers, payment selection)
    let callback_handler = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, BotStorage, State>()
        .branch(
            dptree::case![State::InPhase { phase_id, question_id }]
                .endpoint(user::registration::handle_callback),
        )
        .branch(
            dptree::case![State::AwaitingPayment]
                .endpoint(user::payment::handle_payment_selection),
        );

    // Branch for chat member updates (users joining groups via invite links)
    let chat_member_handler =
        Update::filter_chat_member().endpoint(group::member_join::handle);

    // Branch for bot's own membership changes (auto-register groups when admin adds bot)
    let my_chat_member_handler =
        Update::filter_my_chat_member().endpoint(group::auto_register::handle);

    dptree::entry()
        .branch(message_handler)
        .branch(callback_handler)
        .branch(chat_member_handler)
        .branch(my_chat_member_handler)
}

async fn handle_help(bot: Bot, msg: Message, lang: Arc<RwLock<Lang>>) -> HandlerResult {
    let l = *lang.read().await;
    bot.send_message(msg.chat.id, i18n::help_text(l))
        .await?;
    Ok(())
}
