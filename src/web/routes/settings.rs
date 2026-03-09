use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    db::models::Setting,
    error::{AppError, Result},
    i18n::Lang,
    web::state::WebState,
};

pub async fn list(State(s): State<WebState>) -> Result<Json<HashMap<String, String>>> {
    let rows = sqlx::query_as::<_, Setting>("SELECT * FROM settings ORDER BY key ASC")
        .fetch_all(&s.db)
        .await?;
    let map: HashMap<String, String> = rows.into_iter().map(|r| (r.key, r.value)).collect();
    Ok(Json(map))
}

pub async fn get(Path(key): Path<String>, State(s): State<WebState>) -> Result<Json<Setting>> {
    let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
        .bind(&key)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("setting '{key}' not found")))?;
    Ok(Json(setting))
}

#[derive(Deserialize)]
pub struct UpdateBody {
    pub value: String,
}

pub async fn update(
    Path(key): Path<String>,
    State(s): State<WebState>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<Setting>> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(&key)
    .bind(&body.value)
    .execute(&s.db)
    .await?;

    // Sync in-memory language cache
    if key == "language" {
        let mut guard = s.lang.write().await;
        *guard = Lang::from_code(&body.value);
    }

    let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
        .bind(&key)
        .fetch_one(&s.db)
        .await?;
    Ok(Json(setting))
}

#[derive(Serialize)]
pub struct CachedTokenResponse {
    pub token: Option<String>,
}

pub async fn livepix_token(State(s): State<WebState>) -> Json<CachedTokenResponse> {
    Json(CachedTokenResponse {
        token: s.payment_provider.get_cached_token(),
    })
}
