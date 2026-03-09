use crate::{
    db::{
        models::{Question, QuestionOption},
        DbPool,
    },
    db_execute, db_query_as,
    error::Result,
};

pub async fn list_by_phase(pool: &DbPool, phase_id: i64) -> Result<Vec<Question>> {
    db_query_as!(pool, Question, "SELECT * FROM questions WHERE phase_id = ? ORDER BY position ASC", [phase_id], fetch_all)
        .map_err(Into::into)
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<Question>> {
    db_query_as!(pool, Question, "SELECT * FROM questions WHERE id = ?", [id], fetch_optional)
        .map_err(Into::into)
}

pub async fn first_in_phase(pool: &DbPool, phase_id: i64) -> Result<Option<Question>> {
    db_query_as!(pool, Question, "SELECT * FROM questions WHERE phase_id = ? ORDER BY position ASC LIMIT 1", [phase_id], fetch_optional)
        .map_err(Into::into)
}

pub async fn next_in_phase(
    pool: &DbPool,
    phase_id: i64,
    current_position: i64,
) -> Result<Option<Question>> {
    db_query_as!(pool, Question, "SELECT * FROM questions WHERE phase_id = ? AND position > ? ORDER BY position ASC LIMIT 1", [phase_id, current_position], fetch_optional)
        .map_err(Into::into)
}

pub async fn create(
    pool: &DbPool,
    phase_id: i64,
    text: &str,
    question_type: &str,
    position: i64,
    required: bool,
    media_path: Option<&str>,
    media_type: Option<&str>,
) -> Result<i64> {
    let (id,): (i64,) = db_query_as!(pool, (i64,),
        "INSERT INTO questions (phase_id, text, question_type, position, required, media_path, media_type) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        [phase_id, text, question_type, position, required, media_path, media_type], fetch_one)?;
    Ok(id)
}

pub async fn update(
    pool: &DbPool,
    id: i64,
    text: &str,
    question_type: &str,
    position: i64,
    required: bool,
    media_path: Option<&str>,
    media_type: Option<&str>,
) -> Result<()> {
    db_execute!(pool,
        "UPDATE questions SET text = ?, question_type = ?, position = ?, required = ?, media_path = ?, media_type = ?, media_file_id = NULL WHERE id = ?",
        [text, question_type, position, required, media_path, media_type, id])?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<()> {
    db_execute!(pool, "DELETE FROM questions WHERE id = ?", [id])?;
    Ok(())
}

pub async fn reorder(pool: &DbPool, items: &[(i64, i64)]) -> Result<()> {
    for (id, position) in items {
        db_execute!(pool, "UPDATE questions SET position = ? WHERE id = ?", [position, id])?;
    }
    Ok(())
}

pub async fn update_media_file_id(pool: &DbPool, id: i64, file_id: &str) -> Result<()> {
    db_execute!(pool, "UPDATE questions SET media_file_id = ? WHERE id = ?", [file_id, id])?;
    Ok(())
}

// --- Options ---

pub async fn list_options(pool: &DbPool, question_id: i64) -> Result<Vec<QuestionOption>> {
    db_query_as!(pool, QuestionOption, "SELECT * FROM question_options WHERE question_id = ? ORDER BY position ASC", [question_id], fetch_all)
        .map_err(Into::into)
}

pub async fn get_option_by_id(pool: &DbPool, id: i64) -> Result<Option<QuestionOption>> {
    db_query_as!(pool, QuestionOption, "SELECT * FROM question_options WHERE id = ?", [id], fetch_optional)
        .map_err(Into::into)
}

pub async fn create_option(
    pool: &DbPool,
    question_id: i64,
    label: &str,
    value: &str,
    position: i64,
) -> Result<i64> {
    let (id,): (i64,) = db_query_as!(pool, (i64,),
        "INSERT INTO question_options (question_id, label, value, position) VALUES (?, ?, ?, ?) RETURNING id",
        [question_id, label, value, position], fetch_one)?;
    Ok(id)
}

pub async fn update_option(
    pool: &DbPool,
    id: i64,
    label: &str,
    value: &str,
    position: i64,
) -> Result<()> {
    db_execute!(pool, "UPDATE question_options SET label = ?, value = ?, position = ? WHERE id = ?", [label, value, position, id])?;
    Ok(())
}

pub async fn delete_option(pool: &DbPool, id: i64) -> Result<()> {
    db_execute!(pool, "DELETE FROM question_options WHERE id = ?", [id])?;
    Ok(())
}
