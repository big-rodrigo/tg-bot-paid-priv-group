use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    bot,
    db::models::Setting,
    db_execute, db_query_as,
    error::{AppError, Result},
    i18n::Lang,
    web::state::WebState,
};

pub async fn list(State(s): State<WebState>) -> Result<Json<HashMap<String, String>>> {
    let rows = db_query_as!(&s.db, Setting, "SELECT * FROM settings ORDER BY key ASC", [], fetch_all)?;
    let map: HashMap<String, String> = rows.into_iter().map(|r| (r.key, r.value)).collect();
    Ok(Json(map))
}

pub async fn get(Path(key): Path<String>, State(s): State<WebState>) -> Result<Json<Setting>> {
    let setting = db_query_as!(&s.db, Setting, "SELECT * FROM settings WHERE key = ?", [&key], fetch_optional)?
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
    db_execute!(&s.db,
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [&key, &body.value])?;

    // Sync in-memory language cache and re-register bot commands with Telegram
    if key == "bot_language" {
        let lang = Lang::from_code(&body.value);
        *s.lang.write().await = lang;
        bot::set_bot_commands(&s.bot, lang).await;
    }

    let setting = db_query_as!(&s.db, Setting, "SELECT * FROM settings WHERE key = ?", [&key], fetch_one)?;
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
