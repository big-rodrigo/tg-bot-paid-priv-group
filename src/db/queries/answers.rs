use chrono::NaiveDateTime;
use serde::Serialize;

use crate::{
    db::DbPool,
    db_execute, db_query_as,
    error::Result,
};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EnrichedAnswer {
    pub answer_id: i64,
    pub text_value: Option<String>,
    pub option_id: Option<i64>,
    pub image_file_id: Option<String>,
    pub answered_at: NaiveDateTime,
    pub question_id: i64,
    pub question_text: String,
    pub question_type: String,
    pub question_position: i64,
    pub phase_id: i64,
    pub phase_name: String,
    pub phase_position: i64,
    pub option_label: Option<String>,
}

pub async fn save_text(pool: &DbPool, user_id: i64, question_id: i64, text: &str) -> Result<()> {
    db_execute!(pool,
        "INSERT INTO answers (user_id, question_id, text_value)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET text_value = excluded.text_value",
        [user_id, question_id, text])?;
    Ok(())
}

pub async fn save_option(pool: &DbPool, user_id: i64, question_id: i64, option_id: i64) -> Result<()> {
    db_execute!(pool,
        "INSERT INTO answers (user_id, question_id, option_id)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET option_id = excluded.option_id",
        [user_id, question_id, option_id])?;
    Ok(())
}

pub async fn save_image(pool: &DbPool, user_id: i64, question_id: i64, file_id: &str) -> Result<()> {
    db_execute!(pool,
        "INSERT INTO answers (user_id, question_id, image_file_id)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET image_file_id = excluded.image_file_id",
        [user_id, question_id, file_id])?;
    Ok(())
}

pub async fn list_enriched_by_user(pool: &DbPool, user_id: i64) -> Result<Vec<EnrichedAnswer>> {
    db_query_as!(pool, EnrichedAnswer,
        "SELECT
            a.id            AS answer_id,
            a.text_value,
            a.option_id,
            a.image_file_id,
            a.created_at    AS answered_at,
            q.id            AS question_id,
            q.text          AS question_text,
            q.question_type AS question_type,
            q.position      AS question_position,
            p.id            AS phase_id,
            p.name          AS phase_name,
            p.position      AS phase_position,
            qo.label        AS option_label
        FROM answers a
        JOIN questions q ON q.id = a.question_id
        JOIN phases p ON p.id = q.phase_id
        LEFT JOIN question_options qo ON qo.id = a.option_id
        WHERE a.user_id = ?
        ORDER BY p.position ASC, q.position ASC",
        [user_id], fetch_all)
        .map_err(Into::into)
}
