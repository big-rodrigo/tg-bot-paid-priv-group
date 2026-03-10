use teloxide::utils::command::BotCommands;

/// Commands available only to the configured admin user.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Admin commands:")]
pub enum AdminCommand {
    #[command(description = "Show admin help")]
    Admin,
    #[command(description = "List all registered users")]
    Users,
    #[command(description = "List configured groups")]
    Groups,
    #[command(description = "List active phases")]
    Phases,
    #[command(description = "Send invite links to a user: /sendinvites <telegram_id>")]
    Sendinvites(String),
}
