pub mod restore;

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use chrono::Utc;
use teloxide::{prelude::*, types::InputFile};
use tokio::sync::RwLock;

use crate::{
    config::AppConfig,
    db::DbPool,
    db_query_scalar,
    i18n::{self, Lang},
};

const MAX_CHUNK_SIZE: u64 = 49 * 1024 * 1024; // 49 MB (Telegram limit is 50 MB)

/// Guard that resets the `running` flag on drop (even on panic/error).
struct RunGuard<'a>(&'a AtomicBool);
impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

pub struct BackupManager {
    pool: DbPool,
    bot: Bot,
    config: Arc<AppConfig>,
    lang: Arc<RwLock<Lang>>,
    running: AtomicBool,
}

impl BackupManager {
    pub fn new(
        pool: DbPool,
        bot: Bot,
        config: Arc<AppConfig>,
        lang: Arc<RwLock<Lang>>,
    ) -> Self {
        Self {
            pool,
            bot,
            config,
            lang,
            running: AtomicBool::new(false),
        }
    }

    /// Returns true if a backup is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Quick check called from bot handlers. If a backup is due, spawns a
    /// background task to create + send it. Never blocks the caller.
    pub fn maybe_trigger(self: &Arc<Self>) {
        // Fast path: already running
        if self.running.load(Ordering::Relaxed) {
            return;
        }

        let mgr = Arc::clone(self);
        tokio::spawn(async move {
            if let Err(e) = mgr.maybe_trigger_inner().await {
                tracing::warn!("Backup trigger check failed: {e}");
            }
        });
    }

    async fn maybe_trigger_inner(&self) -> anyhow::Result<()> {
        // Read settings
        let enabled: Option<String> = db_query_scalar!(
            &self.pool, String,
            "SELECT value FROM settings WHERE key = 'backup_enabled'",
            [], fetch_optional
        )?;
        if enabled.as_deref() != Some("true") {
            return Ok(());
        }

        let admin_chat_id: Option<String> = db_query_scalar!(
            &self.pool, String,
            "SELECT value FROM settings WHERE key = 'admin_chat_id'",
            [], fetch_optional
        )?;
        let admin_chat_id = match admin_chat_id {
            Some(id) => id.parse::<i64>().unwrap_or(0),
            None => return Ok(()),
        };
        if admin_chat_id == 0 {
            return Ok(());
        }

        let interval_str: Option<String> = db_query_scalar!(
            &self.pool, String,
            "SELECT value FROM settings WHERE key = 'backup_interval_hours'",
            [], fetch_optional
        )?;
        let interval_hours: i64 = interval_str
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24);

        let last_backup_str: Option<String> = db_query_scalar!(
            &self.pool, String,
            "SELECT value FROM settings WHERE key = 'last_backup_at'",
            [], fetch_optional
        )?;

        if let Some(ref ts) = last_backup_str {
            if let Ok(last) = chrono::NaiveDateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S") {
                let elapsed = Utc::now().naive_utc() - last;
                if elapsed < chrono::Duration::hours(interval_hours) {
                    return Ok(());
                }
            }
        }

        // Try to claim the slot
        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(());
        }

        // Run the backup (the guard ensures `running` is reset)
        let _guard = RunGuard(&self.running);
        self.run_backup(admin_chat_id).await
    }

    /// Manually trigger a backup. Returns Err if one is already running.
    pub async fn trigger_manual(&self) -> anyhow::Result<()> {
        let admin_chat_id: Option<String> = db_query_scalar!(
            &self.pool, String,
            "SELECT value FROM settings WHERE key = 'admin_chat_id'",
            [], fetch_optional
        )?;
        let admin_chat_id = admin_chat_id
            .and_then(|s| s.parse::<i64>().ok())
            .filter(|&id| id != 0)
            .ok_or_else(|| anyhow::anyhow!("Admin chat ID not set. Send /admin to the bot first."))?;

        if self
            .running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(anyhow::anyhow!("A backup is already in progress"));
        }

        let _guard = RunGuard(&self.running);
        self.run_backup(admin_chat_id).await
    }

    async fn run_backup(&self, admin_chat_id: i64) -> anyhow::Result<()> {
        let chat_id = ChatId(admin_chat_id);
        let l = *self.lang.read().await;

        tracing::info!("Starting database backup");

        let result = self.run_backup_inner(chat_id, l).await;

        match &result {
            Ok(()) => {
                tracing::info!("Database backup completed successfully");
            }
            Err(e) => {
                tracing::error!("Database backup failed: {e}");
                let _ = self
                    .bot
                    .send_message(chat_id, i18n::backup_failed(l, &e.to_string()))
                    .await;
            }
        }

        result
    }

    async fn run_backup_inner(&self, chat_id: ChatId, l: Lang) -> anyhow::Result<()> {
        // 1. Create raw backup
        let raw_path = self.create_backup().await?;

        // 2. Compress
        let gz_path = compress(&raw_path).await?;

        // 3. Split if needed
        let parts = split_if_needed(&gz_path).await?;
        let total = parts.len();

        // 4. Send to admin
        let now = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
        for (i, part_path) in parts.iter().enumerate() {
            let caption = if total == 1 {
                i18n::backup_caption(l, &now)
            } else {
                i18n::backup_part_caption(l, &now, i + 1, total)
            };

            let filename = part_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            self.bot
                .send_document(chat_id, InputFile::file(part_path).file_name(filename))
                .caption(caption)
                .await?;
        }

        // 5. Update last_backup_at
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        crate::db_execute!(
            &self.pool,
            "INSERT INTO settings (key, value) VALUES ('last_backup_at', ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [&timestamp]
        )?;

        // 6. Clean up temp files
        for part in &parts {
            let _ = tokio::fs::remove_file(part).await;
        }
        // gz_path might already be removed if no splitting happened
        let _ = tokio::fs::remove_file(&gz_path).await;
        let _ = tokio::fs::remove_file(&raw_path).await;

        Ok(())
    }

    async fn create_backup(&self) -> anyhow::Result<PathBuf> {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let tmp_dir = std::env::temp_dir();

        match &self.pool {
            DbPool::Sqlite(sqlite_pool) => {
                let filename = format!("backup_{timestamp}.db");
                let backup_path = tmp_dir.join(&filename);
                let path_str = backup_path
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid temp path"))?
                    .to_string();

                // VACUUM INTO creates a clean snapshot without locking the live DB
                sqlx::query(&format!("VACUUM INTO '{}'", path_str.replace('\'', "''")))
                    .execute(sqlite_pool)
                    .await?;

                Ok(backup_path)
            }
            DbPool::Postgres(_) => {
                let filename = format!("backup_{timestamp}.sql");
                let backup_path = tmp_dir.join(&filename);

                let output = tokio::process::Command::new("pg_dump")
                    .arg(&format!("--dbname={}", self.config.database_url))
                    .arg("--format=plain")
                    .arg("--no-owner")
                    .arg("--no-acl")
                    .output()
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to run pg_dump (is it installed?): {e}"
                        )
                    })?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("pg_dump failed: {stderr}"));
                }

                tokio::fs::write(&backup_path, &output.stdout).await?;
                Ok(backup_path)
            }
        }
    }
}

/// Compress a file with gzip, returning the path to the .gz file.
async fn compress(path: &Path) -> anyhow::Result<PathBuf> {
    let gz_path = path.with_extension(
        format!(
            "{}.gz",
            path.extension().unwrap_or_default().to_string_lossy()
        ),
    );

    let input_path = path.to_path_buf();
    let output_path = gz_path.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let input = std::fs::File::open(&input_path)?;
        let output = std::fs::File::create(&output_path)?;
        let mut encoder =
            flate2::write::GzEncoder::new(output, flate2::Compression::default());
        let mut reader = std::io::BufReader::new(input);
        std::io::copy(&mut reader, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    })
    .await??;

    // Remove the uncompressed original
    let _ = tokio::fs::remove_file(path).await;

    Ok(gz_path)
}

/// Split a file into ≤ MAX_CHUNK_SIZE parts if it exceeds that size.
/// Returns the list of part files (or the original file if small enough).
async fn split_if_needed(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.len() <= MAX_CHUNK_SIZE {
        return Ok(vec![path.to_path_buf()]);
    }

    let input_path = path.to_path_buf();
    let parts = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<PathBuf>> {
        let mut reader = std::fs::File::open(&input_path)?;
        let mut parts = Vec::new();
        let mut part_num = 1u32;
        let mut buf = vec![0u8; 8 * 1024 * 1024]; // 8 MB read buffer

        loop {
            let part_path = input_path.with_extension(format!("part{:02}", part_num));
            let mut writer = std::fs::File::create(&part_path)?;
            let mut written: u64 = 0;

            loop {
                let to_read =
                    ((MAX_CHUNK_SIZE - written) as usize).min(buf.len());
                if to_read == 0 {
                    break;
                }
                let n = reader.read(&mut buf[..to_read])?;
                if n == 0 {
                    break;
                }
                writer.write_all(&buf[..n])?;
                written += n as u64;
            }

            if written == 0 {
                // No more data — remove the empty file
                let _ = std::fs::remove_file(&part_path);
                break;
            }

            parts.push(part_path);
            part_num += 1;

            if written < MAX_CHUNK_SIZE {
                break; // Last chunk was smaller → we're done
            }
        }

        // Remove the original unsplit file
        let _ = std::fs::remove_file(&input_path);

        Ok(parts)
    })
    .await??;

    Ok(parts)
}
