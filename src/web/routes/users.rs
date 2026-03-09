use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    db::queries,
    db_query_as,
    error::{AppError, Result},
    web::state::WebState,
};

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_page() -> i64 { 1 }
fn default_limit() -> i64 { 50 }

pub async fn list(
    State(s): State<WebState>,
    Query(p): Query<Pagination>,
) -> Result<Json<serde_json::Value>> {
    let users = queries::users::list(&s.db, p.page, p.limit).await?;
    Ok(Json(serde_json::json!(users)))
}

pub async fn get(
    State(s): State<WebState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let user = queries::users::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {id} not found")))?;

    let reg = db_query_as!(&s.db, crate::db::models::UserRegistration,
        "SELECT * FROM user_registration WHERE user_id = ?",
        [id], fetch_optional)?;

    Ok(Json(serde_json::json!({
        "user": user,
        "registration": reg,
    })))
}

pub async fn get_answers(
    State(s): State<WebState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let answers = queries::answers::list_enriched_by_user(&s.db, id).await?;
    Ok(Json(serde_json::json!(answers)))
}

pub async fn get_invite_links(
    State(s): State<WebState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let links = queries::invite_links::list_for_user(&s.db, id).await?;
    Ok(Json(serde_json::json!(links)))
}
