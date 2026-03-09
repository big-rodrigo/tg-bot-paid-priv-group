use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::RwLock;

use crate::{
    bot::{commands::AdminCommand, state::HandlerResult},
    config::AppConfig,
    db::{queries, DbPool},
    i18n::{self, Lang},
};

pub async fn handle(
    bot: Bot,
    msg: Message,
    cmd: AdminCommand,
    pool: DbPool,
    _config: Arc<AppConfig>,
    lang: Arc<RwLock<Lang>>,
) -> HandlerResult {
    let l = *lang.read().await;
    match cmd {
        AdminCommand::Admin => {
            bot.send_message(msg.chat.id, i18n::admin_help_text(l)).await?;
        }

        AdminCommand::Users => {
            let users = queries::users::list(&pool, 1, 20).await?;
            if users.is_empty() {
                bot.send_message(msg.chat.id, i18n::admin_no_users(l)).await?;
            } else {
                let text = users
                    .iter()
                    .map(|u| {
                        format!(
                            "• {} (@{}) — id: {}",
                            u.first_name,
                            u.username.as_deref().unwrap_or(i18n::admin_no_username(l)),
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
                bot.send_message(msg.chat.id, i18n::admin_no_groups(l))
                    .await?;
            } else {
                let text = groups
                    .iter()
                    .map(|g| {
                        format!(
                            "• {} (id: {}, {}: {})",
                            g.title,
                            g.telegram_id,
                            if g.active { i18n::admin_active(l) } else { i18n::admin_inactive(l) },
                            g.active
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
                bot.send_message(msg.chat.id, i18n::admin_no_phases(l))
                    .await?;
            } else {
                let text = phases
                    .iter()
                    .map(|p| {
                        format!(
                            "{}. {} [{}]",
                            p.position,
                            p.name,
                            if p.active { i18n::admin_active(l) } else { i18n::admin_inactive(l) }
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
                    bot.send_message(msg.chat.id, i18n::admin_usage_sendinvites(l))
                        .await?;
                    return Ok(());
                }
            };
            let user = queries::users::get_by_telegram_id(&pool, telegram_id).await?;
            match user {
                None => {
                    bot.send_message(msg.chat.id, i18n::admin_user_not_found(l)).await?;
                }
                Some(user) => {
                    crate::bot::user::invite::deliver_invites(
                        bot.clone(),
                        pool.clone(),
                        user.id,
                        user.telegram_id,
                        l,
                    )
                    .await?;
                    bot.send_message(msg.chat.id, i18n::admin_invites_sent(l, telegram_id))
                        .await?;
                }
            }
        }
    }

    Ok(())
}
