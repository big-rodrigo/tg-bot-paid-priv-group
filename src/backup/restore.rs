use std::{collections::{HashMap, HashSet, VecDeque}, path::Path};

use crate::db::DbPool;

/// Restore a database from a backup file.
///
/// Detects format first:
/// - JSON (`{...}`) — works for both SQLite and PostgreSQL (cross-restore supported)
/// - SQLite `.db` binary — SQLite only, uses ATTACH DATABASE
/// - SQL text dump — PostgreSQL only, uses psql subprocess
pub async fn restore_database(pool: &DbPool, backup_path: &Path) -> anyhow::Result<()> {
    let content = tokio::fs::read(backup_path).await?;
    let is_json = content.first().map_or(false, |&b| b == b'{');

    if is_json {
        let text = String::from_utf8(content)
            .map_err(|e| anyhow::anyhow!("Invalid UTF-8 in backup file: {e}"))?;
        return match pool {
            DbPool::Sqlite(sqlite_pool) => restore_json_to_sqlite(sqlite_pool, &text).await,
            DbPool::Postgres(pg_pool) => restore_postgres_json(pg_pool, &text).await,
        };
    }

    // Non-JSON: dispatch to format-specific restore
    match pool {
        DbPool::Sqlite(sqlite_pool) => restore_sqlite(sqlite_pool, backup_path).await,
        DbPool::Postgres(pg_pool) => restore_postgres_sql(pg_pool, backup_path).await,
    }
}

// ── SQLite native restore (ATTACH) ───────────────────────────────────────────

async fn restore_sqlite(
    pool: &sqlx::SqlitePool,
    backup_path: &Path,
) -> anyhow::Result<()> {
    let path_str = backup_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid backup path"))?
        .to_string();

    sqlx::query(&format!(
        "ATTACH DATABASE '{}' AS backup_db",
        path_str.replace('\'', "''")
    ))
    .execute(pool)
    .await?;

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM backup_db.sqlite_master WHERE type='table' \
         AND name NOT LIKE '_sqlx%' AND name NOT LIKE 'sqlite_%'"
    )
    .fetch_all(pool)
    .await?;

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    for (table_name,) in &tables {
        if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
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

    sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await?;
    sqlx::query("DETACH DATABASE backup_db").execute(pool).await?;

    Ok(())
}

// ── JSON restore → SQLite ─────────────────────────────────────────────────────

async fn restore_json_to_sqlite(
    pool: &sqlx::SqlitePool,
    content: &str,
) -> anyhow::Result<()> {
    let backup: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| anyhow::anyhow!("Invalid JSON backup: {e}"))?;

    let tables_obj = backup
        .get("tables")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Invalid backup: missing 'tables' object"))?;

    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    // Delete in reverse-dependency order, then insert forward.
    // For simplicity on SQLite (FK already disabled), just process all tables.
    for (table_name, rows_val) in tables_obj {
        if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            tracing::warn!("Skipping table with invalid name: {table_name}");
            continue;
        }

        sqlx::query(&format!("DELETE FROM \"{table_name}\""))
            .execute(pool)
            .await?;

        let rows = match rows_val.as_array() {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };

        for row in rows {
            let obj = match row.as_object() {
                Some(o) if !o.is_empty() => o,
                _ => continue,
            };

            let cols: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
            let col_list = cols
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            let values_list = cols
                .iter()
                .map(|col| json_to_sqlite_literal(&obj[*col]))
                .collect::<Vec<_>>()
                .join(", ");

            sqlx::query(&format!(
                "INSERT INTO \"{table_name}\" ({col_list}) VALUES ({values_list})"
            ))
            .execute(pool)
            .await?;
        }
    }

    sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await?;

    Ok(())
}

/// Convert a JSON value to a SQLite SQL literal (safe for inline SQL).
fn json_to_sqlite_literal(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        // Arrays/objects shouldn't appear in our schema, but handle gracefully
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

// ── PostgreSQL SQL dump restore (psql) ───────────────────────────────────────

async fn restore_postgres_sql(
    _pool: &sqlx::PgPool,
    backup_path: &Path,
) -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL not available"))?;

    let sql_content = tokio::fs::read(backup_path).await?;

    let psql_result = tokio::process::Command::new("psql")
        .arg(&database_url)
        .arg("--single-transaction")
        .arg("--quiet")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match psql_result {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(&sql_content).await?;
            }
            let output = child.wait_with_output().await?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("psql restore failed: {stderr}"));
            }
            Ok(())
        }
        Err(_) => Err(anyhow::anyhow!(
            "Cannot restore SQL dump: psql is not installed. \
             Use a JSON-format backup instead."
        )),
    }
}

// ── JSON restore → PostgreSQL ─────────────────────────────────────────────────

async fn restore_postgres_json(
    pool: &sqlx::PgPool,
    content: &str,
) -> anyhow::Result<()> {
    let backup: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| anyhow::anyhow!("Invalid JSON backup: {e}"))?;

    let tables_obj = backup
        .get("tables")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Invalid backup: missing 'tables' object"))?;

    let backup_tables: Vec<String> = tables_obj.keys().cloned().collect();

    for name in &backup_tables {
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(anyhow::anyhow!("Invalid table name in backup: {name}"));
        }
    }

    let delete_order = get_topological_order(pool, &backup_tables).await?;
    let insert_order: Vec<String> = delete_order.iter().rev().cloned().collect();

    let can_disable_triggers = sqlx::query_scalar::<_, String>(
        "SELECT current_setting('is_superuser')"
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map_or(false, |v| v == "on");

    let mut tx = pool.begin().await?;

    if can_disable_triggers {
        for table in &backup_tables {
            let _ = sqlx::query(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
                .execute(&mut *tx)
                .await;
        }
    }

    for table in &delete_order {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await?;
    }

    for table in &insert_order {
        let rows = match tables_obj.get(table).and_then(|v| v.as_array()) {
            Some(r) if !r.is_empty() => r,
            _ => continue,
        };

        let json_array = serde_json::Value::Array(rows.clone());
        let json_str = json_array.to_string();

        sqlx::query(&format!(
            "INSERT INTO {table} SELECT * FROM json_populate_recordset(NULL::{table}, $1::json)"
        ))
        .bind(&json_str)
        .execute(&mut *tx)
        .await?;
    }

    if can_disable_triggers {
        for table in &backup_tables {
            let _ = sqlx::query(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
                .execute(&mut *tx)
                .await;
        }
    }

    tx.commit().await?;

    Ok(())
}

// ── FK topological sort ───────────────────────────────────────────────────────

async fn get_topological_order(
    pool: &sqlx::PgPool,
    tables: &[String],
) -> anyhow::Result<Vec<String>> {
    let deps: Vec<(String, String)> = sqlx::query_as(
        "SELECT tc.table_name, ccu.table_name AS referenced_table \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.constraint_column_usage ccu \
           ON tc.constraint_name = ccu.constraint_name \
           AND tc.table_schema = ccu.table_schema \
         WHERE tc.constraint_type = 'FOREIGN KEY' \
           AND tc.table_schema = 'public'"
    )
    .fetch_all(pool)
    .await?;

    let table_set: HashSet<&str> = tables.iter().map(|s| s.as_str()).collect();

    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for t in &table_set {
        dependents.entry(t).or_default();
        in_degree.entry(t).or_insert(0);
    }

    for (child, parent) in &deps {
        if table_set.contains(child.as_str())
            && table_set.contains(parent.as_str())
            && child != parent
        {
            dependents
                .entry(parent.as_str())
                .or_default()
                .push(child.as_str());
            *in_degree.entry(child.as_str()).or_insert(0) += 1;
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&t, _)| t)
        .collect();

    let mut parent_first = Vec::new();

    while let Some(node) = queue.pop_front() {
        parent_first.push(node.to_string());
        if let Some(children) = dependents.get(node) {
            for &child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(child);
                    }
                }
            }
        }
    }

    for t in tables {
        if !parent_first.contains(t) {
            parent_first.push(t.clone());
        }
    }

    // Reverse → children-first (safe for DELETE)
    parent_first.reverse();
    Ok(parent_first)
}
