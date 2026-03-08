use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::{
    bot::{commands::AdminCommand, state::HandlerResult},
    config::AppConfig,
    db::{queries, DbPool},
};

pub async fn handle(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    pool: DbPool,
    _config: Arc<AppConfig>,
) -> HandlerResult {
    match cmd {
        AdminCommand::Admin => {
            let text = AdminCommand::descriptions().to_string();
            bot.send_message(msg.chat.id, text).await?;
        }

        AdminCommand::Users => {
            let users = queries::users::list(&pool, 1, 20).await?;
            if users.is_empty() {
                bot.send_message(msg.chat.id, "No registered users yet.").await?;
            } else {
                let text = users
                    .iter()
                    .map(|u| {
                        format!(
                            "• {} (@{}) — id: {}",
                            u.first_name,
                            u.username.as_deref().unwrap_or("no username"),
                            u.telegram_id,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                bot.send_message(msg.chat.id, format!("Users:\n{text}")).await?;
            }
        }

        AdminCommand::Groups => {
            use crate::db::models::Group;
            let groups = sqlx::query_as::<_, Group>(
                "SELECT * FROM groups ORDER BY id ASC",
            )
            .fetch_all(&pool)
            .await?;
            if groups.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No groups configured. Add them via the web interface.",
                )
                .await?;
            } else {
                let text = groups
                    .iter()
                    .map(|g| {
                        format!(
                            "• {} (id: {}, active: {})",
                            g.title, g.telegram_id, g.active
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                bot.send_message(msg.chat.id, format!("Groups:\n{text}")).await?;
            }
        }

        AdminCommand::Phases => {
            let phases = queries::phases::list_all(&pool).await?;
            if phases.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "No phases configured. Add them via the web interface.",
                )
                .await?;
            } else {
                let text = phases
                    .iter()
                    .map(|p| {
                        format!(
                            "{}. {} [{}]",
                            p.position,
                            p.name,
                            if p.active { "active" } else { "inactive" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                bot.send_message(msg.chat.id, format!("Phases:\n{text}")).await?;
            }
        }

        AdminCommand::Sendinvites(args) => {
            let telegram_id: i64 = match args.trim().parse() {
                Ok(id) => id,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Usage: /sendinvites <telegram_id>")
                        .await?;
                    return Ok(());
                }
            };
            let user = queries::users::get_by_telegram_id(&pool, telegram_id).await?;
            match user {
                None => {
                    bot.send_message(msg.chat.id, "User not found.").await?;
                }
                Some(user) => {
                    crate::bot::user::invite::deliver_invites(
                        bot.clone(),
                        pool.clone(),
                        user.id,
                        user.telegram_id,
                    )
                    .await?;
                    bot.send_message(
                        msg.chat.id,
                        format!("Invite links sent to user {}.", telegram_id),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}
