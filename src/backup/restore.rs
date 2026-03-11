use std::path::Path;

use crate::db::DbPool;

/// Restore a database from a backup file.
///
/// - SQLite: uses ATTACH DATABASE to copy all tables from the backup into the live DB.
/// - PostgreSQL: pipes the SQL dump through `psql` in a single transaction.
pub async fn restore_database(pool: &DbPool, backup_path: &Path) -> anyhow::Result<()> {
    match pool {
        DbPool::Sqlite(sqlite_pool) => restore_sqlite(sqlite_pool, backup_path).await,
        DbPool::Postgres(_) => restore_postgres(pool, backup_path).await,
    }
}

async fn restore_sqlite(
    pool: &sqlx::SqlitePool,
    backup_path: &Path,
) -> anyhow::Result<()> {
    let path_str = backup_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid backup path"))?
        .to_string();

    // Attach the uploaded backup database
    sqlx::query(&format!(
        "ATTACH DATABASE '{}' AS backup_db",
        path_str.replace('\'', "''")
    ))
    .execute(pool)
    .await?;

    // Get all table names from the backup (excluding internal SQLite and sqlx tables)
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM backup_db.sqlite_master WHERE type='table' AND name NOT LIKE '_sqlx%' AND name NOT LIKE 'sqlite_%'"
    )
    .fetch_all(pool)
    .await?;

    // Disable foreign keys for the duration of the restore
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    // For each table: clear live data and copy from backup
    for (table_name,) in &tables {
        // Validate table name to prevent SQL injection (only alphanumeric + underscore)
        if !table_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_')
        {
            tracing::warn!("Skipping table with invalid name: {table_name}");
            continue;
        }

        sqlx::query(&format!("DELETE FROM main.{table_name}"))
            .execute(pool)
            .await?;

        sqlx::query(&format!(
            "INSERT INTO main.{table_name} SELECT * FROM backup_db.{table_name}"
        ))
        .execute(pool)
        .await?;
    }

    // Re-enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    // Detach the backup database
    sqlx::query("DETACH DATABASE backup_db")
        .execute(pool)
        .await?;

    Ok(())
}

async fn restore_postgres(pool: &DbPool, backup_path: &Path) -> anyhow::Result<()> {
    let database_url = match pool {
        DbPool::Postgres(_) => {
            // We need the DATABASE_URL to pass to psql.
            // Read it from the environment since AppConfig stores it.
            std::env::var("DATABASE_URL")
                .map_err(|_| anyhow::anyhow!("DATABASE_URL not available for psql"))?
        }
        _ => unreachable!(),
    };

    let sql_content = tokio::fs::read(backup_path).await?;

    let mut child = tokio::process::Command::new("psql")
        .arg(&database_url)
        .arg("--single-transaction")
        .arg("--quiet")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to run psql (is it installed?): {e}"))?;

    // Write the SQL dump to psql's stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(&sql_content).await?;
        // Drop stdin to signal EOF
    }

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("psql restore failed: {stderr}"));
    }

    Ok(())
}
