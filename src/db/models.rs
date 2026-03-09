use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    pub id: i64,
    pub telegram_id: i64,
    pub title: String,
    pub active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Phase {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub position: i64,
    pub active: bool,
    pub phase_type: String,
    pub rejection_text: Option<String>,
    pub clean_chat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Question {
    pub id: i64,
    pub phase_id: i64,
    pub text: String,
    pub question_type: String,
    pub position: i64,
    pub required: bool,
    pub media_path: Option<String>,
    pub media_type: Option<String>,
    pub media_file_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QuestionOption {
    pub id: i64,
    pub question_id: i64,
    pub label: String,
    pub value: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Answer {
    pub id: i64,
    pub user_id: i64,
    pub question_id: i64,
    pub text_value: Option<String>,
    pub option_id: Option<i64>,
    pub image_file_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRegistration {
    pub user_id: i64,
    pub current_phase_id: Option<i64>,
    pub current_question_id: Option<i64>,
    pub completed_at: Option<NaiveDateTime>,
    pub first_message_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub external_ref: Option<String>,
    pub telegram_charge_id: Option<String>,
    pub amount: Option<i64>,
    pub currency: Option<String>,
    pub price_cents: Option<i64>,
    pub status: String,
    pub payload: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InviteRule {
    pub id: i64,
    pub phase_id: i64,
    pub group_id: i64,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentGateCondition {
    pub id: i64,
    pub phase_id: i64,
    pub question_id: i64,
    pub condition_type: String,
    pub option_id: Option<i64>,
    pub text_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InviteRuleCondition {
    pub id: i64,
    pub invite_rule_id: i64,
    pub question_id: i64,
    pub condition_type: String,
    pub option_id: Option<i64>,
    pub text_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct InviteLink {
    pub id: i64,
    pub user_id: i64,
    pub group_id: i64,
    pub invite_link: String,
    pub created_at: NaiveDateTime,
    pub used_at: Option<NaiveDateTime>,
    pub revoked_at: Option<NaiveDateTime>,
}
