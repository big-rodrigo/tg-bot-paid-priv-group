pub mod auth;
pub mod routes;
pub mod state;

use axum::{
    extract::DefaultBodyLimit,
    http::Method,
    middleware,
    routing::{delete, get, post, put},
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
            "/api/phases/{id}",
            put(routes::phases::update).delete(routes::phases::delete),
        )
        // Questions
        .route(
            "/api/phases/{phase_id}/questions",
            get(routes::questions::list_by_phase).post(routes::questions::create),
        )
        .route(
            "/api/questions/{id}",
            put(routes::questions::update).delete(routes::questions::delete),
        )
        .route("/api/questions/reorder", put(routes::questions::reorder))
        // Options
        .route(
            "/api/questions/{question_id}/options",
            get(routes::questions::list_options).post(routes::questions::create_option),
        )
        .route(
            "/api/options/{id}",
            put(routes::questions::update_option).delete(routes::questions::delete_option),
        )
        // Invite Rules (static routes before {id} param routes)
        .route(
            "/api/invite-rules/reorder",
            put(routes::invite_rules::reorder),
        )
        .route(
            "/api/invite-rules/questions",
            get(routes::invite_rules::available_questions),
        )
        .route(
            "/api/phases/{phase_id}/invite-rules",
            get(routes::invite_rules::list_by_phase).post(routes::invite_rules::create),
        )
        .route(
            "/api/invite-rules/{id}",
            put(routes::invite_rules::update).delete(routes::invite_rules::delete),
        )
        .route(
            "/api/invite-rules/{invite_rule_id}/conditions",
            get(routes::invite_rules::list_conditions).post(routes::invite_rules::create_condition),
        )
        .route(
            "/api/invite-rule-conditions/{id}",
            delete(routes::invite_rules::delete_condition),
        )
        // Payment gate conditions
        .route(
            "/api/phases/{phase_id}/gate-conditions",
            get(routes::payment_gate::list).post(routes::payment_gate::create),
        )
        .route(
            "/api/gate-conditions/{id}",
            delete(routes::payment_gate::delete),
        )
        // Groups
        .route(
            "/api/groups",
            get(routes::groups::list).post(routes::groups::create),
        )
        .route(
            "/api/groups/{id}",
            put(routes::groups::update).delete(routes::groups::delete),
        )
        // Users
        .route("/api/users", get(routes::users::list))
        .route("/api/users/{id}", get(routes::users::get))
        .route("/api/users/{id}/answers", get(routes::users::get_answers))
        .route(
            "/api/users/{id}/invite_links",
            get(routes::users::get_invite_links),
        )
        // Payments
        .route("/api/payments", get(routes::payments::list))
        .route("/api/payments/{id}/complete", post(routes::payments::complete))
        // Settings
        .route("/api/settings", get(routes::settings::list))
        .route(
            "/api/settings/{key}",
            get(routes::settings::get).put(routes::settings::update),
        )
        .route("/api/debug/livepix-token", get(routes::settings::livepix_token))
        // Admin actions
        .route(
            "/api/admin/send-invites/{user_id}",
            post(routes::admin::send_invites),
        )
        .route(
            "/api/admin/revoke-links/{user_id}",
            post(routes::admin::revoke_links),
        )
        .route(
            "/api/admin/reset-registration/{user_id}",
            post(routes::admin::reset_registration),
        )
        .route(
            "/api/admin/unregister/{user_id}",
            post(routes::admin::unregister),
        )
        .route(
            "/api/admin/telegram-image",
            get(routes::admin::telegram_image),
        )
        // File upload
        .route(
            "/api/upload",
            post(routes::upload::upload_file)
                .delete(routes::upload::delete_file)
                .layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .layer(middleware::from_fn_with_state(state.clone(), auth::basic_auth));

    // ── Uploaded media files (served before SPA fallback) ────────────────
    let uploads = Router::new().nest_service("/uploads", ServeDir::new("uploads"));

    // ── SPA fallback — serves static/ (built Svelte app) ─────────────────
    let spa = ServeDir::new("static").fallback(ServeFile::new("static/index.html"));

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(uploads)
        .fallback_service(spa)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
