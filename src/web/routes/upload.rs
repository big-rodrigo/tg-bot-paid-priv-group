use axum::{extract::Multipart, Json};
use std::path::Path;
use uuid::Uuid;

use crate::error::{AppError, Result};

const MAX_FILE_SIZE: usize = 20 * 1024 * 1024; // 20 MB
const UPLOADS_DIR: &str = "uploads";

fn media_type_from_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "image/gif" => Some("animation"),
        m if m.starts_with("image/") => Some("image"),
        m if m.starts_with("video/") => Some("video"),
        _ => None,
    }
}

fn extension_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        _ => "bin",
    }
}

pub async fn upload_file(
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart data: {e}")))?
        .ok_or_else(|| AppError::BadRequest("No file field provided".into()))?;

    let mime = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let media_type = media_type_from_mime(&mime)
        .ok_or_else(|| AppError::BadRequest(format!("Unsupported file type: {mime}")))?;

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to read file: {e}")))?;

    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest(format!(
            "File too large ({} bytes). Max: {} bytes",
            data.len(),
            MAX_FILE_SIZE
        )));
    }

    let ext = extension_from_mime(&mime);
    let filename = format!("{}.{}", Uuid::new_v4(), ext);
    let relative_path = format!("{UPLOADS_DIR}/{filename}");

    tokio::fs::create_dir_all(UPLOADS_DIR)
        .await
        .map_err(|e| AppError::Other(format!("Failed to create uploads dir: {e}")))?;

    tokio::fs::write(&relative_path, &data)
        .await
        .map_err(|e| AppError::Other(format!("Failed to write file: {e}")))?;

    Ok(Json(serde_json::json!({
        "media_path": relative_path,
        "media_type": media_type,
    })))
}

#[derive(serde::Deserialize)]
pub struct DeleteUpload {
    pub media_path: String,
}

pub async fn delete_file(
    Json(body): Json<DeleteUpload>,
) -> Result<axum::http::StatusCode> {
    let path = Path::new(&body.media_path);

    // Validate path is within uploads directory
    if !body.media_path.starts_with(UPLOADS_DIR) {
        return Err(AppError::BadRequest("Invalid media path".into()));
    }

    // Ignore errors (file may already be gone)
    let _ = tokio::fs::remove_file(path).await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
