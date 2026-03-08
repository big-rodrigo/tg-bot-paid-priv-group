# tg-bot — Telegram Private Groups Access Manager

## Overview

Single Rust binary that manages paid access to private Telegram groups. Users go through an admin-configured multi-phase registration, pay, and receive unique one-time invite links to all configured groups. The admin configures phases, questions, and groups through a local web interface (Svelte SPA) and via Telegram commands.

## Quick start

```bash
cp .env.example .env
# Required: BOT_API_KEY, ADMIN_TELEGRAM_USERNAME, WEB_INTERFACE_SECRET
cargo run
```

```bash
# Admin web interface (optional — build once, then served by the bot)
cd frontend && npm install && npm run build
# visit http://localhost:3000  (admin / <WEB_INTERFACE_SECRET>)
```

## Environment variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `BOT_API_KEY` | yes | — | Telegram Bot token from @BotFather |
| `ADMIN_TELEGRAM_USERNAME` | yes | — | Admin username without @ |
| `WEB_INTERFACE_SECRET` | yes | — | Basic auth password for web UI |
| `DATABASE_URL` | no | `sqlite:./data.db` | SQLite or Postgres URL |
| `PAYMENT_API_URL` | no | — | External payment API endpoint |
| `PAYMENT_API_KEY` | no | — | Bearer token for external payment API |
| `TELEGRAM_PAYMENT_PROVIDER_TOKEN` | no | — | Enables Telegram native payments |
| `WEB_INTERFACE_PORT` | no | `3000` | Port for web interface |
| `RUST_LOG` | no | `info` | Tracing filter (e.g. `tg_bot=debug,tower_http=info`) |

## Architecture

Three concurrent Tokio tasks in one binary:

```
main.rs
├── tokio::spawn → bot::run_dispatcher()     Teloxide dispatcher
├── tokio::spawn → axum::serve()             Axum web server
└── tokio::select! → exit if either crashes
```

Shared state is passed via `Arc<T>`:
- `DbPool` (`sqlx::SqlitePool`) — database connection pool
- `Bot` — Telegram bot client (cheaply cloneable)
- `Arc<AppConfig>` — parsed env vars
- `Arc<dyn PaymentProvider + Send + Sync>` — selected payment provider

## Module map

```
src/
├── main.rs               Entry point: connect DB, spawn tasks
├── config.rs             AppConfig struct (parsed from env by `envy`)
├── error.rs              AppError enum + axum IntoResponse impl
│
├── db/
│   ├── mod.rs            DbPool type alias (SqlitePool)
│   ├── models.rs         All sqlx::FromRow structs (User, Phase, Question, …)
│   └── queries/          Async fns for each table (no query! macros)
│       ├── users.rs
│       ├── phases.rs
│       ├── questions.rs  Includes option CRUD
│       ├── answers.rs
│       ├── payments.rs
│       └── invite_links.rs
│
├── bot/
│   ├── mod.rs            Dispatcher setup + dptree handler tree
│   ├── state.rs          State enum, BotStorage, BotDialogue, HandlerResult
│   ├── commands.rs       UserCommand + AdminCommand enums (BotCommands derive)
│   ├── admin/
│   │   ├── guards.rs     is_admin() filter — checks username vs ADMIN_TELEGRAM_USERNAME
│   │   └── commands.rs   /admin /users /groups /phases /sendinvites handlers
│   ├── group/
│   │   ├── member_join.rs     ChatMemberUpdated handler — marks invite link used
│   │   └── invite_manager.rs  create_one_time_link(), revoke_link()
│   └── user/
│       ├── welcome.rs         /start handler — upserts user, starts phase flow
│       ├── registration.rs    Phase/question handlers + send_question() helper
│       ├── payment.rs         Payment selection keyboard, pre_checkout, successful_payment
│       └── invite.rs          deliver_invites() — called after payment confirmed
│
├── payment/
│   ├── mod.rs            PaymentProvider trait + PaymentInitiation + WebhookEvent
│   ├── external.rs       POSTs to PAYMENT_API_URL, verifies Bearer token on webhook
│   └── telegram.rs       Telegram Payments scaffold (invoice sent from bot handler)
│
└── web/
    ├── mod.rs            create_router() — assembles protected + public routes + SPA
    ├── state.rs          WebState { db, bot, config, payment_provider }
    ├── auth.rs           Basic auth middleware (admin:<WEB_INTERFACE_SECRET>)
    └── routes/
        ├── phases.rs     GET/POST /api/phases, PUT /api/phases/reorder
        ├── questions.rs  Questions + options CRUD
        ├── groups.rs     Groups CRUD
        ├── users.rs      Users list + answers + invite links
        ├── payments.rs   Payments list + POST /api/webhooks/payment (no auth)
        └── admin.rs      POST /api/admin/send-invites/:id, /revoke-links/:id
```

## Bot dialogue state machine

```
Start
  │ /start
  ▼
InPhase { phase_id, question_id }
  │ answer (text / image / button callback)
  │ advance through questions → phases
  ▼
AwaitingPayment
  │ user picks payment method
  ├─[external]──► AwaitingExternalPayment { payment_id }
  │                  webhook POST /api/webhooks/payment
  │                  → deliver_invites()
  └─[telegram]──► AwaitingTelegramPayment { payment_id }
                     successful_payment update
                     → deliver_invites()
                          ▼
                       Registered
```

Storage: `InMemStorage<State>` — resets on restart. For persistence, implement the
`teloxide::dispatching::dialogue::Storage` trait backed by `DbPool`.

## Database

- `DbPool = sqlx::SqlitePool` (defined in `src/db/mod.rs`)
- Runtime queries only — no `query!` macros, so no compile-time `DATABASE_URL` needed
- Migrations run automatically at startup from `migrations/`

**Tables:** `users`, `groups`, `phases`, `questions`, `question_options`, `answers`,
`user_registration`, `payments`, `invite_links`

**To switch to PostgreSQL / Supabase:**
1. Change `pub type DbPool = sqlx::SqlitePool;` → `sqlx::PgPool` in `src/db/mod.rs`
2. Change `sqlx::SqlitePool::connect(...)` → `sqlx::PgPool::connect(...)` in `src/main.rs`
3. Update `DATABASE_URL` to `postgres://user:pass@host/db`
4. In `Cargo.toml`: swap `"sqlite"` for `"postgres"` in sqlx features

## Web API

All `/api/*` routes require `Authorization: Basic base64(admin:<WEB_INTERFACE_SECRET>)`.
Exception: `POST /api/webhooks/payment` is public (verified by Bearer token from payment provider).

```
Phases:       GET/POST /api/phases
              PUT      /api/phases/reorder          [{id, position}]
              PUT/DEL  /api/phases/:id

Questions:    GET/POST /api/phases/:phase_id/questions
              PUT/DEL  /api/questions/:id
              PUT      /api/questions/reorder

Options:      GET/POST /api/questions/:question_id/options
              PUT/DEL  /api/options/:id

Groups:       GET/POST/PUT/DEL /api/groups, /api/groups/:id

Users:        GET /api/users                        ?page=&limit=
              GET /api/users/:id
              GET /api/users/:id/answers
              GET /api/users/:id/invite_links

Payments:     GET  /api/payments                    ?status=
              POST /api/webhooks/payment             (no auth)

Admin:        POST /api/admin/send-invites/:user_id
              POST /api/admin/revoke-links/:user_id
```

The Svelte SPA is served from `static/` as a fallback. Build it with:
```bash
cd frontend && npm run build   # outputs to static/
```

## Adding a registration flow

Via the web API (or admin UI at `http://localhost:3000`):
```bash
# Create a phase
POST /api/phases  {"name":"Screening","position":0}

# Add a text question
POST /api/phases/1/questions  {"text":"Why do you want to join?","question_type":"text","position":0}

# Add a button question with options
POST /api/phases/1/questions  {"text":"Age range?","question_type":"button","position":1}
POST /api/questions/2/options  {"label":"18-25","value":"18_25","position":0}
POST /api/questions/2/options  {"label":"26-35","value":"26_35","position":1}

# Add an image question
POST /api/phases/1/questions  {"text":"Upload a photo ID","question_type":"image","position":2}
```

## Adding a payment provider

1. Create `src/payment/my_provider.rs` implementing the `PaymentProvider` trait:
   ```rust
   #[async_trait]
   impl PaymentProvider for MyProvider {
       async fn initiate(&self, user: &User, payment_id: i64) -> Result<PaymentInitiation> { ... }
       async fn verify_webhook(&self, headers: &HeaderMap, body: &Bytes) -> Result<WebhookEvent> { ... }
       fn provider_name(&self) -> &'static str { "my_provider" }
   }
   ```
2. Add a new env var (e.g. `MY_PROVIDER_KEY`) to `AppConfig` in `src/config.rs`
3. Wire it up in the provider selection block in `src/main.rs`

## Group setup

The bot must be an **administrator** in every private group with permission to create invite links.
Add the group's Telegram ID (negative number, e.g. `-1001234567890`) via:
```bash
POST /api/groups  {"telegram_id": -1001234567890, "title": "My Private Group"}
```

## Key conventions

- Error propagation: all fallible ops return `crate::error::Result<T>`; `AppError` implements `axum::response::IntoResponse`
- dptree dependency injection: handler function parameters are resolved by type from the dispatcher context — add new shared deps via `dptree::deps![...]` in `src/bot/mod.rs`
- Admin guard: `src/bot/admin/guards.rs::is_admin()` — case-insensitive username match
- Invite links: created with `member_limit=1`, stored in `invite_links` table, marked `used_at` when `ChatMemberUpdated` arrives in `group/member_join.rs`
- DB: use `sqlx::query_as::<_, ModelType>(sql).bind(...).fetch_*(pool)` pattern throughout

## Frontend design system

**Stack:** Svelte 5.0.0, Vite 6.0.0, TypeScript 5.0.0 (strict). No UI component library — all styling is scoped CSS in `.svelte` files. Build output goes to `static/` (served by the Rust backend).

### Color palette

All colors are hardcoded hex values; no CSS custom properties are used.

| Role | Hex | Usage |
|---|---|---|
| Primary / dark | `#1a1a2e` | Nav bar, buttons, selected state, headings |
| Nav link | `#aad4f5` | Navigation anchor text |
| Background | `#f5f5f5` | Page body background |
| Surface | `#ffffff` | Cards, tables, form inputs |
| Text primary | `#333` | Body text |
| Text secondary | `#666` / `#888` | Labels, helper text |
| Border | `#ddd` / `#ccc` | Input borders, table dividers |
| Subtle bg | `#f0f0f0` / `#f9f9f9` | Table headers, muted sections |
| Hover tint | `#f0f4ff` | Row / list item hover background |
| Danger | `#c0392b` | Delete buttons, error / failed badges |
| Warning | `#e67e22` | Warn buttons, pending payment badges |
| Success | `#27ae60` | Completed payments, used invite links |
| Muted | `#7f8c8d` | Refunded / disabled state badges |

### Patterns

**Routing:** Hash-based SPA (`#/path`). `App.svelte` listens to `hashchange` and maps paths to page components via a plain object — no routing library.

**State:** Svelte 5 runes only (`$state`, `$derived`). State is local to each page component; no global store.

**API layer:** All requests go through `src/lib/api.ts` — a typed fetch wrapper that injects Basic Auth from the `localStorage`-stored secret. Functions are grouped by resource (`phases.list()`, `users.get()`, etc.).

**Layouts:**
- Master-detail (sidebar 300px fixed + 1fr main) — Users page
- Three-column cascade (phases → questions → options) — Phases page
- Full-width table, `max-width: 1100px` — Groups, Payments

**UI conventions:**
- Selected list items: `border-left: 3px solid #1a1a2e`
- Buttons: default navy `#1a1a2e`; add class `.danger` (red) or `.warn` (orange); `border-radius: 4px`
- Status badges: pill-shaped (`border-radius: 10px`), colored by status (pending/completed/failed/refunded)
- Typography: system font stack (`-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif`); no external fonts
- Spacing: 0.25–1.5rem scale for gaps and padding
- Border radius: 4px on inputs/buttons, 6–8px on cards/panels
- Shadows: subtle only — `box-shadow: 0 1px 3px rgba(0,0,0,.07)`

## Known limitations

- Dialogue state is in-memory — lost on process restart
- External payment API body/response shape is a stub in `src/payment/external.rs` — adjust when the real API is defined
- Telegram Payments amount/currency is hardcoded to `$10.00 USD` in `src/bot/user/payment.rs`
