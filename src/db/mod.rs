pub mod models;
pub mod queries;

/// Database connection pool supporting both SQLite and PostgreSQL.
///
/// The backend is selected automatically based on the `DATABASE_URL` env var:
///   - `sqlite:./data.db` → SQLite
///   - `postgres://user:pass@host/db` → PostgreSQL
#[derive(Clone, Debug)]
pub enum DbPool {
    Sqlite(sqlx::SqlitePool),
    Postgres(sqlx::PgPool),
}

/// Convert `?` placeholders to `$1, $2, …` for PostgreSQL.
pub fn pg_placeholders(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len() + 16);
    let mut n = 0u32;
    let mut in_quote = false;
    for ch in sql.chars() {
        if ch == '\'' {
            in_quote = !in_quote;
            out.push(ch);
        } else if ch == '?' && !in_quote {
            n += 1;
            out.push('$');
            out.push_str(&n.to_string());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Execute `sqlx::query_as` dispatching to the correct backend.
///
/// Usage: `db_query_as!(pool, Type, "SQL", [bind1, bind2], fetch_all)`
#[macro_export]
macro_rules! db_query_as {
    ($pool:expr, $T:ty, $sql:expr, [$($bind:expr),* $(,)?], $fetch:ident) => {
        match $pool {
            $crate::db::DbPool::Sqlite(ref __p) => {
                sqlx::query_as::<_, $T>($sql)
                    $(.bind($bind))*
                    .$fetch(__p)
                    .await
            }
            $crate::db::DbPool::Postgres(ref __p) => {
                let __sql = $crate::db::pg_placeholders($sql);
                sqlx::query_as::<_, $T>(&__sql)
                    .persistent(false)
                    $(.bind($bind))*
                    .$fetch(__p)
                    .await
            }
        }
    };
}

/// Execute `sqlx::query` (no result rows) dispatching to the correct backend.
///
/// Usage: `db_execute!(pool, "SQL", [bind1, bind2])`
#[macro_export]
macro_rules! db_execute {
    ($pool:expr, $sql:expr, [$($bind:expr),* $(,)?]) => {
        match $pool {
            $crate::db::DbPool::Sqlite(ref __p) => {
                sqlx::query($sql)
                    $(.bind($bind))*
                    .execute(__p)
                    .await
                    .map(|_| ())
            }
            $crate::db::DbPool::Postgres(ref __p) => {
                let __sql = $crate::db::pg_placeholders($sql);
                sqlx::query(&__sql)
                    .persistent(false)
                    $(.bind($bind))*
                    .execute(__p)
                    .await
                    .map(|_| ())
            }
        }
    };
}

/// Execute `sqlx::query_scalar` dispatching to the correct backend.
///
/// Usage: `db_query_scalar!(pool, String, "SQL", [bind1], fetch_optional)`
#[macro_export]
macro_rules! db_query_scalar {
    ($pool:expr, $T:ty, $sql:expr, [$($bind:expr),* $(,)?], $fetch:ident) => {
        match $pool {
            $crate::db::DbPool::Sqlite(ref __p) => {
                sqlx::query_scalar::<_, $T>($sql)
                    $(.bind($bind))*
                    .$fetch(__p)
                    .await
            }
            $crate::db::DbPool::Postgres(ref __p) => {
                let __sql = $crate::db::pg_placeholders($sql);
                sqlx::query_scalar::<_, $T>(&__sql)
                    .persistent(false)
                    $(.bind($bind))*
                    .$fetch(__p)
                    .await
            }
        }
    };
}
