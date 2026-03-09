use crate::{
    db::{models::Phase, DbPool},
    db_execute, db_query_as,
    error::Result,
};

pub async fn list_active(pool: &DbPool) -> Result<Vec<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases WHERE active = TRUE ORDER BY position ASC", [], fetch_all)
        .map_err(Into::into)
}

pub async fn list_active_normal(pool: &DbPool) -> Result<Vec<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases WHERE active = TRUE AND phase_type = 'normal' ORDER BY position ASC", [], fetch_all)
        .map_err(Into::into)
}

pub async fn list_active_invite(pool: &DbPool) -> Result<Vec<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases WHERE active = TRUE AND phase_type = 'invite' ORDER BY position ASC", [], fetch_all)
        .map_err(Into::into)
}

pub async fn get_active_payment(pool: &DbPool) -> Result<Option<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases WHERE active = TRUE AND phase_type = 'payment' ORDER BY position ASC LIMIT 1", [], fetch_optional)
        .map_err(Into::into)
}

pub async fn list_all(pool: &DbPool) -> Result<Vec<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases ORDER BY position ASC", [], fetch_all)
        .map_err(Into::into)
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<Phase>> {
    db_query_as!(pool, Phase, "SELECT * FROM phases WHERE id = ?", [id], fetch_optional)
        .map_err(Into::into)
}

pub async fn create(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
    position: i64,
    phase_type: &str,
    rejection_text: Option<&str>,
    clean_chat: bool,
) -> Result<i64> {
    let (id,): (i64,) = db_query_as!(pool, (i64,),
        "INSERT INTO phases (name, description, position, phase_type, rejection_text, clean_chat) VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        [name, description, position, phase_type, rejection_text, clean_chat], fetch_one)?;
    Ok(id)
}

pub async fn update(
    pool: &DbPool,
    id: i64,
    name: &str,
    description: Option<&str>,
    position: i64,
    active: bool,
    phase_type: &str,
    rejection_text: Option<&str>,
    clean_chat: bool,
) -> Result<()> {
    db_execute!(pool,
        "UPDATE phases SET name = ?, description = ?, position = ?, active = ?, phase_type = ?, rejection_text = ?, clean_chat = ? WHERE id = ?",
        [name, description, position, active, phase_type, rejection_text, clean_chat, id])?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<()> {
    db_execute!(pool, "DELETE FROM phases WHERE id = ?", [id])?;
    Ok(())
}

pub async fn reorder(pool: &DbPool, items: &[(i64, i64)]) -> Result<()> {
    for (id, position) in items {
        db_execute!(pool, "UPDATE phases SET position = ? WHERE id = ?", [position, id])?;
    }
    Ok(())
}
