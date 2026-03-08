use crate::{
    db::{
        models::{Question, QuestionOption},
        DbPool,
    },
    error::Result,
};

pub async fn list_by_phase(pool: &DbPool, phase_id: i64) -> Result<Vec<Question>> {
    sqlx::query_as::<_, Question>(
        "SELECT * FROM questions WHERE phase_id = ? ORDER BY position ASC",
    )
    .bind(phase_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<Question>> {
    sqlx::query_as::<_, Question>("SELECT * FROM questions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn first_in_phase(pool: &DbPool, phase_id: i64) -> Result<Option<Question>> {
    sqlx::query_as::<_, Question>(
        "SELECT * FROM questions WHERE phase_id = ? ORDER BY position ASC LIMIT 1",
    )
    .bind(phase_id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn next_in_phase(
    pool: &DbPool,
    phase_id: i64,
    current_position: i64,
) -> Result<Option<Question>> {
    sqlx::query_as::<_, Question>(
        "SELECT * FROM questions WHERE phase_id = ? AND position > ? ORDER BY position ASC LIMIT 1",
    )
    .bind(phase_id)
    .bind(current_position)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn create(
    pool: &DbPool,
    phase_id: i64,
    text: &str,
    question_type: &str,
    position: i64,
    required: bool,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO questions (phase_id, text, question_type, position, required) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(phase_id)
    .bind(text)
    .bind(question_type)
    .bind(position)
    .bind(required)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn update(
    pool: &DbPool,
    id: i64,
    text: &str,
    question_type: &str,
    position: i64,
    required: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE questions SET text = ?, question_type = ?, position = ?, required = ? WHERE id = ?",
    )
    .bind(text)
    .bind(question_type)
    .bind(position)
    .bind(required)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM questions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder(pool: &DbPool, items: &[(i64, i64)]) -> Result<()> {
    for (id, position) in items {
        sqlx::query("UPDATE questions SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

// --- Options ---

pub async fn list_options(pool: &DbPool, question_id: i64) -> Result<Vec<QuestionOption>> {
    sqlx::query_as::<_, QuestionOption>(
        "SELECT * FROM question_options WHERE question_id = ? ORDER BY position ASC",
    )
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn create_option(
    pool: &DbPool,
    question_id: i64,
    label: &str,
    value: &str,
    position: i64,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO question_options (question_id, label, value, position) VALUES (?, ?, ?, ?)",
    )
    .bind(question_id)
    .bind(label)
    .bind(value)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn update_option(
    pool: &DbPool,
    id: i64,
    label: &str,
    value: &str,
    position: i64,
) -> Result<()> {
    sqlx::query("UPDATE question_options SET label = ?, value = ?, position = ? WHERE id = ?")
        .bind(label)
        .bind(value)
        .bind(position)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_option(pool: &DbPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM question_options WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
