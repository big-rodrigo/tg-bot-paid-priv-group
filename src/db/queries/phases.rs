use crate::{
    db::{models::Phase, DbPool},
    error::Result,
};

pub async fn list_active(pool: &DbPool) -> Result<Vec<Phase>> {
    sqlx::query_as::<_, Phase>(
        "SELECT * FROM phases WHERE active = TRUE ORDER BY position ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_all(pool: &DbPool) -> Result<Vec<Phase>> {
    sqlx::query_as::<_, Phase>("SELECT * FROM phases ORDER BY position ASC")
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

pub async fn get_by_id(pool: &DbPool, id: i64) -> Result<Option<Phase>> {
    sqlx::query_as::<_, Phase>("SELECT * FROM phases WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}

pub async fn create(pool: &DbPool, name: &str, description: Option<&str>, position: i64) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO phases (name, description, position) VALUES (?, ?, ?)",
    )
    .bind(name)
    .bind(description)
    .bind(position)
    .execute(pool)
    .await?;
    Ok(row.last_insert_rowid())
}

pub async fn update(
    pool: &DbPool,
    id: i64,
    name: &str,
    description: Option<&str>,
    position: i64,
    active: bool,
) -> Result<()> {
    sqlx::query(
        "UPDATE phases SET name = ?, description = ?, position = ?, active = ? WHERE id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(position)
    .bind(active)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &DbPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM phases WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reorder(pool: &DbPool, items: &[(i64, i64)]) -> Result<()> {
    for (id, position) in items {
        sqlx::query("UPDATE phases SET position = ? WHERE id = ?")
            .bind(position)
            .bind(id)
            .execute(pool)
            .await?;
    }
    Ok(())
}
