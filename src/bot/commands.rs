use teloxide::utils::command::BotCommands;

/// Commands available to all users.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum UserCommand {
    #[command(description = "Start the registration process")]
    Start,
    #[command(description = "Show this help message")]
    Help,
    #[command(description = "Check your registration status")]
    Status,
    #[command(description = "Get your invite links again")]
    Mylinks,
}

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
