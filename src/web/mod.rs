pub mod auth;
pub mod routes;
pub mod state;

use axum::{
    http::Method,
    middleware,
    routing::{get, post, put},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use self::state::WebState;

pub fn create_router(state: WebState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // ── Public routes (no auth) ──────────────────────────────────────────
    let public = Router::new()
        .route("/api/webhooks/payment", post(routes::payments::webhook));

    // ── Protected routes (Basic auth required) ───────────────────────────
    let protected = Router::new()
        // Phases
        .route("/api/phases", get(routes::phases::list).post(routes::phases::create))
        .route("/api/phases/reorder", put(routes::phases::reorder))
        .route(
            "/api/phases/:id",
            put(routes::phases::update).delete(routes::phases::delete),
        )
        // Questions
        .route(
            "/api/phases/:phase_id/questions",
            get(routes::questions::list_by_phase).post(routes::questions::create),
        )
        .route(
            "/api/questions/:id",
            put(routes::questions::update).delete(routes::questions::delete),
        )
        .route("/api/questions/reorder", put(routes::questions::reorder))
        // Options
        .route(
            "/api/questions/:question_id/options",
            get(routes::questions::list_options).post(routes::questions::create_option),
        )
        .route(
            "/api/options/:id",
            put(routes::questions::update_option).delete(routes::questions::delete_option),
        )
        // Groups
        .route(
            "/api/groups",
            get(routes::groups::list).post(routes::groups::create),
        )
        .route(
            "/api/groups/:id",
            put(routes::groups::update).delete(routes::groups::delete),
        )
        // Users
        .route("/api/users", get(routes::users::list))
        .route("/api/users/:id", get(routes::users::get))
        .route("/api/users/:id/answers", get(routes::users::get_answers))
        .route(
            "/api/users/:id/invite_links",
            get(routes::users::get_invite_links),
        )
        // Payments
        .route("/api/payments", get(routes::payments::list))
        // Settings
        .route("/api/settings", get(routes::settings::list))
        .route(
            "/api/settings/:key",
            get(routes::settings::get).put(routes::settings::update),
        )
        .route("/api/debug/livepix-token", get(routes::settings::livepix_token))
        // Admin actions
        .route(
            "/api/admin/send-invites/:user_id",
            post(routes::admin::send_invites),
        )
        .route(
            "/api/admin/revoke-links/:user_id",
            post(routes::admin::revoke_links),
        )
        .layer(middleware::from_fn_with_state(state.clone(), auth::basic_auth));

    // ── SPA fallback — serves static/ (built Svelte app) ─────────────────
    let spa = Router::new().nest_service(
        "/",
        ServeDir::new("static").fallback(ServeFile::new("static/index.html")),
    );

    Router::new()
        .merge(public)
        .merge(protected)
        .fallback_service(spa)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
