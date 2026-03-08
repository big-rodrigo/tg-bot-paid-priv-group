use crate::{
    db::{models::Answer, DbPool},
    error::Result,
};

pub async fn save_text(pool: &DbPool, user_id: i64, question_id: i64, text: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO answers (user_id, question_id, text_value)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET text_value = excluded.text_value",
    )
    .bind(user_id)
    .bind(question_id)
    .bind(text)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn save_option(pool: &DbPool, user_id: i64, question_id: i64, option_id: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO answers (user_id, question_id, option_id)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET option_id = excluded.option_id",
    )
    .bind(user_id)
    .bind(question_id)
    .bind(option_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn save_image(pool: &DbPool, user_id: i64, question_id: i64, file_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO answers (user_id, question_id, image_file_id)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, question_id) DO UPDATE SET image_file_id = excluded.image_file_id",
    )
    .bind(user_id)
    .bind(question_id)
    .bind(file_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_by_user(pool: &DbPool, user_id: i64) -> Result<Vec<Answer>> {
    sqlx::query_as::<_, Answer>(
        "SELECT * FROM answers WHERE user_id = ? ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
