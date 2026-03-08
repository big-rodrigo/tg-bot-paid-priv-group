pub mod models;
pub mod queries;

/// Database connection pool.
///
/// Uses `SqlitePool` by default (DATABASE_URL = `sqlite:./data.db`).
///
/// To switch to PostgreSQL / Supabase:
///   1. Change the type alias below to `sqlx::PgPool`
///   2. Set DATABASE_URL = `postgres://user:pass@host/db`
///   3. Replace `.connect` in main.rs with `sqlx::PgPool::connect`
///   4. Add "postgres" to sqlx features in Cargo.toml and remove "sqlite"
pub type DbPool = sqlx::SqlitePool;
