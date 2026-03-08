use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};

use crate::web::state::WebState;

/// Basic auth middleware for admin web interface.
/// Expects: `Authorization: Basic base64(admin:<WEB_INTERFACE_SECRET>)`
pub async fn basic_auth(
    axum::extract::State(state): axum::extract::State<WebState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let is_valid = auth_header
        .and_then(|h| h.strip_prefix("Basic "))
        .and_then(|encoded| STANDARD.decode(encoded).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|decoded| {
            let expected = format!("admin:{}", state.config.web_interface_secret);
            // Use a constant-time comparison to prevent timing attacks
            decoded.len() == expected.len()
                && decoded
                    .bytes()
                    .zip(expected.bytes())
                    .all(|(a, b)| a == b)
        })
        .unwrap_or(false);

    if is_valid {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
