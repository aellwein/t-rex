# Architecture

t-rex is a time recording app. A Rust server serves a REST API and a single-page web app. A command line client talks to the same API.

## Crates

All crates live under `crates/`. The root `Cargo.toml` defines the workspace and its members.

| Crate     | Role |
|-----------|------|
| `crates/dto/`    | Request and response types, validation and calendar rules. Shared by the server and the WASM web app, so it has no server dependencies. |
| `crates/server/` | Axum HTTP server: REST API under `/api/v1`, OIDC login, persistence through SeaORM, serves the web app. |
| `crates/webapp/` | Dioxus single-page app compiled to WASM. Responsive, runs in the browser. |
| `crates/cli/`    | The `t-rex` command line client (clap). Calls the REST API with a token. |

Inside `crates/server/src`:

| Module       | Role |
|--------------|------|
| `main.rs`    | Reads the OIDC settings, connects and migrates the database, builds the router and starts listening. |
| `config.rs`  | Reads the database settings from the environment (`DATABASE_URL`, `T_REX_DB_PATH`, ...). |
| `db.rs`      | Opens the connection (SQLite or Postgres) and runs the migrations. |
| `api/`       | HTTP handlers, one module per resource. Turns store results into JSON and errors into status codes. |
| `store/`     | Database queries. Handlers call the store; the store knows nothing about HTTP. |
| `entity/`    | SeaORM models of the tables. |
| `migration/` | Schema changes as SeaORM migrations. |
| `embed.rs`   | Serves the embedded web app and redirects signed-out browsers to sign-in. |

## Request flow

```
browser ──► server ──► OIDC auth layer (session cookie, ID token)
                ├── /api/v1/...     REST handlers ──► store ──► database
                └── everything else  embedded web app (index.html, WASM, assets)

t-rex CLI ──► server ──► /api/v1/... with "Authorization: Bearer trx_..."
```

Every route runs behind the OIDC authentication layer. The layer keeps the ID token per browser session on the server. Handlers ask for the current account through an extractor, so they never see a request without one.

## Authentication

- **Browser:** OpenID Connect through `axum-oidc-client`. `/auth` starts the sign-in, `/auth/callback` completes it. The session lifetime is set in `crates/server/src/main.rs` (`with_session_max_age`).
- **Command line:** a token. The CLI opens the browser sign-in, receives a one-time code on a local port, and exchanges it for a token. See [cli.md](cli.md#sign-in).
- **Tokens:** the server generates `trx_` followed by 32 random bytes encoded as base64url. It stores only the SHA-256 hash, so a database leak does not reveal usable tokens. A token is refused after its expiry (10 days).
- **Which extractor to use:** `CurrentAccount` accepts a browser session or a token. `SessionAccount` accepts only a browser session. Creating tokens and starting the CLI sign-in use `SessionAccount`, so a leaked token cannot mint new ones.

### Whose data is it?

Each account's data is keyed by one handle:

- the verified email address, lowercased, if the provider marks it as verified;
- otherwise the OIDC subject (`sub`).

An unverified email is never used as a key, because someone could claim an address they do not own.

## Storage

- SeaORM 2 over sqlx. The database is SQLite by default and Postgres when `DATABASE_URL` names one. See [configuration.md](configuration.md#database).
- Every table has a `user_id` column, and every query filters by it. Handlers pass the current user's handle into the store.
- Tables: `projects`, `absence_kinds`, `time_entries`, `settings`, `pending_starts`, `holidays`, `api_tokens`, `cli_login_codes`.
- Migrations run at startup and are tracked in `seaql_migrations`. The baseline schema is hand-written SQL in `crates/server/migrations/seaorm/<database>/`, one file per database, included into the migration with `include_str!`. This keeps CHECK constraints and keys readable.
- Later schema changes are migrations in `crates/server/src/migration/`. Migrations must stay compatible with the previous app version, because an update can run both versions at once.

### Concurrency

- **Token limit:** creating a token removes expired tokens, counts the rest and inserts, in one transaction. On Postgres an advisory lock per user makes concurrent logins for the same account wait for each other. On SQLite the first write serializes the transaction.
- **Login codes:** a code is stored as a hash and used up by deleting its row. Two requests with the same code cannot both succeed, and the code works on any server instance that shares the database.

## Domain rules

The rules live in `crates/dto/` so the server and the web app agree on them.

- **Time record:** a project or absence on one day, with a start minute, a booked work time and an unpaid break. The record occupies `work + break` minutes from its start. Start, end and duration are related by `end = start + duration + break`; any two are enough (`resolve_block`).
- **No overlaps:** records on the same day never share a minute, breaks included. The server checks this on create and update (`overlaps_any`). Records without a start time, from before start times existed, never conflict.
- **Working days:** Monday to Friday, without public holidays (`is_working_day`).
- **Targets and overtime:** the daily target is `daily_minutes` on working days and 0 otherwise. Actual time is project work plus paid absence. The running overtime starts at the carry-over setting and adds each day's difference (`daily_balances`, `running_overtime`).
- **Breaks:** not counted as work. They are summed per day and per period (`break_total`).
- **Holidays:** days without working time. Imported from iCalendar files (see [api.md](api.md#holidays)); one holiday per day.

## Web app

- Dioxus 0.7 compiled to WASM with `dx`. Routes are in `crates/webapp/src/routes.rs`; one module per page in `crates/webapp/src/pages/`.
- `crates/webapp/src/api.rs` is the only place that calls the REST API. It follows `Page<T>` responses until all items are loaded.
- Shared UI state is a global signal (`STATE` in `crates/webapp/src/common/state.rs`). The header and settings read it.
- Languages: English, German, French, Spanish, Italian, Portuguese, Dutch, Polish, Russian, Chinese and Japanese, plus "system" which follows the browser. Strings are Fluent files in `crates/webapp/i18n/`, loaded through `dioxus-i18n`.
- Themes: built-in and popular colour schemes are defined in `crates/dto/src/theme.rs` and `crates/dto/src/theme/popular.rs`, so the server can validate a theme id.

## Build and embedding

- The server embeds the web app in its binary (the `embed` feature, on by default). `crates/server/build.rs` runs `dx bundle --web --release` into `target/webapp-build` before the server compiles, and `embed.rs` embeds that output with `rust-embed`.
- The web app is rebuilt when its sources or `crates/dto/` change. Set `T_REX_SKIP_WEBAPP` to reuse the last build.
- `build.rs` also records the git version, which `/api/v1/version` and the page footer show.
- The web app build uses its own target directory, so the nested build does not wait for the outer cargo lock.

## Security notes

- Secrets come from the environment. The Postgres password must be set in `DATABASE_PASSWORD`; a password inside `DATABASE_URL` is refused at startup.
- API tokens and login codes are stored as SHA-256 hashes.
- Holiday imports from an address are capped at 5 MB, with a 15 second timeout, and events spanning more than 366 days are rejected.
