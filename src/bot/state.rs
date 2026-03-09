use serde::{Deserialize, Serialize};
use teloxide::dispatching::dialogue::InMemStorage;

/// Persistent dialogue state machine for the user registration flow.
///
/// Uses `InMemStorage` (state is held in RAM, cleared on restart).
/// This keeps the sqlx version used for the main DB independent of teloxide's internal sqlx.
/// For production persistence, swap `InMemStorage<State>` for a custom storage
/// backed by your main `DbPool` (see the teloxide docs for `Storage` trait).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum State {
    /// Initial state — user has not started or issued /start
    #[default]
    Start,

    /// User is answering questions in a registration phase
    InPhase {
        phase_id: i64,
        question_id: i64,
    },

    /// All phases complete, waiting for the user to choose a payment method
    AwaitingPayment,

    /// Payment initiated; waiting for the webhook callback
    AwaitingPaymentConfirmation {
        payment_id: i64,
    },

    /// Registration complete — user has received all invite links
    Registered,
}

pub type BotStorage = InMemStorage<State>;
pub type BotDialogue = teloxide::dispatching::dialogue::Dialogue<State, BotStorage>;
pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub fn create_storage() -> std::sync::Arc<BotStorage> {
    InMemStorage::new()
}
