use std::sync::Arc;
use teloxide::types::Message;

use crate::config::AppConfig;

/// Returns `true` if the message sender is the configured admin user.
/// Used as a `dptree::filter` predicate.
pub fn is_admin(msg: Message, config: Arc<AppConfig>) -> bool {
    msg.from.as_ref()
        .and_then(|u| u.username.as_deref())
        .map(|username| username.eq_ignore_ascii_case(&config.admin_telegram_username))
        .unwrap_or(false)
}
