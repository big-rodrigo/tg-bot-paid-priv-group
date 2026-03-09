# tg-bot — Telegram Private Groups Access Manager

## Overview

Single Rust binary that manages paid access to private Telegram groups. Users go through an admin-configured multi-phase registration, pay via LivePix, and receive unique one-time invite links to all configured groups. The admin configures phases, questions, and groups through a local web interface (Svelte SPA) and via Telegram commands.

## Quick start

```bash
cp .env.example .env
# Required: BOT_API_KEY, ADMIN_TELEGRAM_USERNAME, WEB_INTERFACE_SECRET,
#           LIVEPIX_CLIENT_ID, LIVEPIX_CLIENT_SECRET
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
| `LIVEPIX_CLIENT_ID` | yes | — | LivePix OAuth2 client ID |
| `LIVEPIX_CLIENT_SECRET` | yes | — | LivePix OAuth2 client secret |
| `DATABASE_URL` | no | `sqlite:./data.db` | SQLite or Postgres URL |
| `LIVEPIX_ACCOUNT_URL` | no | — | LivePix donation page URL (fallback when DB setting is not configured) |
| `LIVEPIX_PRICE_CENTS` | no | — | Minimum price in cents (fallback when DB setting is not configured) |
| `LIVEPIX_CURRENCY` | no | — | Currency code e.g. BRL (fallback when DB setting is not configured) |
| `WEB_INTERFACE_PORT` | no | `3000` | Port for web interface |
| `WEBHOOK_BASE_URL` | no | — | Public base URL (e.g. ngrok); logged on startup |
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
- `DbPool` (enum: `SqlitePool` or `PgPool`) — database connection pool
- `Bot` — Telegram bot client (cheaply cloneable)
- `Arc<AppConfig>` — parsed env vars
- `Arc<dyn PaymentProvider + Send + Sync>` — LivePix payment provider

## Module map

```
src/
├── main.rs               Entry point: connect DB, spawn tasks
├── config.rs             AppConfig struct (parsed from env by `envy`)
├── error.rs              AppError enum + axum IntoResponse impl
│
├── db/
│   ├── mod.rs            DbPool enum (Sqlite | Postgres) + dispatch macros
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
│   ├── util.rs           Shared helpers (escape_html)
│   ├── admin/
│   │   ├── guards.rs     is_admin() filter — checks username vs ADMIN_TELEGRAM_USERNAME
│   │   └── commands.rs   /admin /users /groups /phases /sendinvites handlers
│   ├── group/
│   │   ├── member_join.rs     ChatMemberUpdated handler — marks invite link used
│   │   └── invite_manager.rs  create_one_time_link(), revoke_link()
│   └── user/
│       ├── welcome.rs         /start handler — upserts user, starts phase flow
│       ├── registration.rs    Phase/question handlers + send_question() helper
│       ├── media.rs           send_media_or_text(), send_and_cache_file_id()
│       ├── payment.rs         LivePix payment selection keyboard
│       └── invite.rs          deliver_invites() — called after payment confirmed
│
├── payment/
│   ├── mod.rs            PaymentProvider trait + PaymentInitiation + WebhookEvent
│   └── livepix.rs        LivePix OAuth2 provider (only payment provider)
│
└── web/
    ├── mod.rs            create_router() — assembles protected + public routes + SPA
    ├── state.rs          WebState { db, bot, config, payment_provider, lang }
    ├── auth.rs           Basic auth middleware (admin:<WEB_INTERFACE_SECRET>)
    └── routes/
        ├── phases.rs     GET/POST /api/phases, PUT /api/phases/reorder
        ├── questions.rs  Questions + options CRUD
        ├── groups.rs     Groups CRUD
        ├── users.rs      Users list + answers + invite links
        ├── payments.rs   Payments list + POST /api/webhooks/payment (no auth)
        ├── admin.rs      POST /api/admin/send-invites/{id}, /revoke-links/{id}
        ├── settings.rs   GET/PUT /api/settings/{key}, GET /api/debug/livepix-token
        ├── upload.rs     POST/DELETE /api/upload (media file management)
        └── invite_rules.rs  Invite rules + conditions CRUD
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
  │ user clicks "Pay via LivePix"
  ▼
AwaitingPaymentConfirmation { payment_id }
  │ webhook POST /api/webhooks/payment
  │ → deliver_invites()
  ▼
Registered
```

Storage: `InMemStorage<State>` — resets on restart. For persistence, implement the
`teloxide::dispatching::dialogue::Storage` trait backed by `DbPool`.

## Payment — LivePix

LivePix is the sole payment provider. It uses OAuth2 client-credentials for API access.

**Flow:**
1. User completes all registration phases
2. Bot shows "Pay via LivePix" button
3. User clicks → bot sends instructions: open LivePix page, type `@username`, pay ≥ minimum
4. LivePix sends webhook to `POST /api/webhooks/payment`
5. Bot verifies payment via LivePix Messages API, marks payment complete, delivers invite links

**Proactive check:** If the webhook doesn't arrive, `/mylinks` command triggers `check_payment()` which queries the LivePix Messages API directly.

**Settings** (configurable via admin UI or `/api/settings/{key}`):
- `livepix_account_url` — donation page URL
- `livepix_price_cents` — minimum payment in cents
- `livepix_currency` — currency code (e.g. `BRL`)

## Database

- `DbPool` is an enum (`Sqlite(SqlitePool)` / `Postgres(PgPool)`) defined in `src/db/mod.rs`
- Backend selected automatically from `DATABASE_URL` env var — no code changes needed
- Three dispatch macros (`db_query_as!`, `db_execute!`, `db_query_scalar!`) handle per-backend SQL placeholder rewriting
- Runtime queries only — no `query!` macros, so no compile-time `DATABASE_URL` needed
- Migrations in `migrations/sqlite/` and `migrations/postgres/` — run automatically at startup based on detected backend
- No `sqlx::AnyPool` — `NaiveDateTime` doesn't implement `Decode<'_, Any>` in sqlx 0.8

**Tables:** `users`, `groups`, `phases`, `questions`, `question_options`, `answers`,
`user_registration`, `payments`, `invite_links`, `invite_rules`, `invite_rule_conditions`, `settings`

**To switch between SQLite and PostgreSQL:** just change `DATABASE_URL`:
- SQLite: `DATABASE_URL=sqlite:./data.db`
- PostgreSQL: `DATABASE_URL=postgres://user:pass@host/db`

## Web API

All `/api/*` routes require `Authorization: Basic base64(admin:<WEB_INTERFACE_SECRET>)`.
Exception: `POST /api/webhooks/payment` is public (verified by LivePix client ID).

```
Phases:       GET/POST /api/phases
              PUT      /api/phases/reorder          [{id, position}]
              PUT/DEL  /api/phases/{id}

Questions:    GET/POST /api/phases/{phase_id}/questions
              PUT/DEL  /api/questions/{id}
              PUT      /api/questions/reorder

Options:      GET/POST /api/questions/{question_id}/options
              PUT/DEL  /api/options/{id}

Groups:       GET/POST/PUT/DEL /api/groups, /api/groups/{id}

Users:        GET /api/users                        ?page=&limit=
              GET /api/users/{id}
              GET /api/users/{id}/answers
              GET /api/users/{id}/invite_links

Payments:     GET  /api/payments                    ?status=
              POST /api/webhooks/payment             (no auth)

Admin:        POST /api/admin/send-invites/{user_id}
              POST /api/admin/revoke-links/{user_id}

Settings:     GET/PUT /api/settings/{key}
              GET     /api/debug/livepix-token

Upload:       POST/DELETE /api/upload

Invite Rules: GET/POST /api/phases/{id}/invite-rules
              PUT/DEL  /api/invite-rules/{id}
              GET/POST /api/invite-rules/{id}/conditions
              PUT/DEL  /api/invite-rule-conditions/{id}
              GET      /api/invite-rules/questions
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
- HTML escaping: use `crate::bot::util::escape_html()` for embedding user data in Telegram HTML messages
- Media sending: use helpers in `src/bot/user/media.rs` for sending messages with optional media attachments

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
- LivePix settings (account URL, price, currency) must be configured via admin UI before payments work
