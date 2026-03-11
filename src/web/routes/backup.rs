use std::path::PathBuf;

use axum::{
    extract::{Multipart, State},
    Json,
};

use crate::{
    db_query_scalar,
    error::{AppError, Result},
    web::state::WebState,
};

pub async fn trigger(State(s): State<WebState>) -> Result<Json<serde_json::Value>> {
    let mgr = &s.backup_manager;

    if mgr.is_running() {
        return Err(AppError::Other("A backup is already in progress".into()));
    }

    let mgr = s.backup_manager.clone();
    tokio::spawn(async move {
        if let Err(e) = mgr.trigger_manual().await {
            tracing::error!("Manual backup failed: {e}");
        }
    });

    Ok(Json(
        serde_json::json!({ "message": "Backup started" }),
    ))
}

pub async fn status(State(s): State<WebState>) -> Result<Json<serde_json::Value>> {
    let running = s.backup_manager.is_running();

    let last_backup_at: Option<String> = db_query_scalar!(
        &s.db,
        String,
        "SELECT value FROM settings WHERE key = 'last_backup_at'",
        [],
        fetch_optional
    )?;

    Ok(Json(serde_json::json!({
        "running": running,
        "last_backup_at": last_backup_at,
    })))
}

pub async fn restore(
    State(s): State<WebState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let tmp_dir = std::env::temp_dir();

    // Collect all uploaded files
    let mut uploaded_files: Vec<(String, PathBuf)> = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
    {
        let filename = field
            .file_name()
            .unwrap_or("backup")
            .to_string();

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read file: {e}")))?;

        let tmp_path = tmp_dir.join(format!("restore_{}", &filename));
        tokio::fs::write(&tmp_path, &data)
            .await
            .map_err(|e| AppError::Other(format!("Failed to write temp file: {e}")))?;

        uploaded_files.push((filename, tmp_path));
    }

    if uploaded_files.is_empty() {
        return Err(AppError::BadRequest("No files uploaded".into()));
    }

    // Sort by filename for correct part ordering
    uploaded_files.sort_by(|a, b| a.0.cmp(&b.0));

    // If multiple files, concatenate them (split parts)
    let gz_path = if uploaded_files.len() == 1 {
        uploaded_files[0].1.clone()
    } else {
        let concat_path = tmp_dir.join("restore_combined.gz");
        let paths: Vec<PathBuf> = uploaded_files.iter().map(|(_, p)| p.clone()).collect();

        tokio::task::spawn_blocking(move || -> std::result::Result<PathBuf, AppError> {
            let mut output = std::fs::File::create(&concat_path)
                .map_err(|e| AppError::Other(format!("Failed to create concat file: {e}")))?;
            for part_path in &paths {
                let mut input = std::fs::File::open(part_path)
                    .map_err(|e| AppError::Other(format!("Failed to open part: {e}")))?;
                std::io::copy(&mut input, &mut output)
                    .map_err(|e| AppError::Other(format!("Failed to concatenate parts: {e}")))?;
            }
            Ok(concat_path)
        })
        .await
        .map_err(|e| AppError::Other(format!("Concat task failed: {e}")))??
    };

    // Decompress
    let decompressed_path = tmp_dir.join("restore_decompressed");
    let gz_clone = gz_path.clone();
    let dec_clone = decompressed_path.clone();

    tokio::task::spawn_blocking(move || -> std::result::Result<(), AppError> {
        let input = std::fs::File::open(&gz_clone)
            .map_err(|e| AppError::Other(format!("Failed to open gz file: {e}")))?;
        let mut decoder = flate2::read::GzDecoder::new(input);
        let mut output = std::fs::File::create(&dec_clone)
            .map_err(|e| AppError::Other(format!("Failed to create decompressed file: {e}")))?;
        std::io::copy(&mut decoder, &mut output)
            .map_err(|e| AppError::Other(format!("Failed to decompress: {e}")))?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::Other(format!("Decompress task failed: {e}")))??;

    // Restore
    crate::backup::restore::restore_database(&s.db, &decompressed_path)
        .await
        .map_err(|e| AppError::Other(format!("Restore failed: {e}")))?;

    // Clean up temp files
    for (_, path) in &uploaded_files {
        let _ = tokio::fs::remove_file(path).await;
    }
    let _ = tokio::fs::remove_file(&gz_path).await;
    let _ = tokio::fs::remove_file(&decompressed_path).await;

    Ok(Json(
        serde_json::json!({ "message": "Database restored successfully" }),
    ))
}
