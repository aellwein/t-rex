# Configuration

The server reads its settings from environment variables. A `.env` file in the working directory is loaded too, if it exists. Keep secrets out of the repository and out of `DATABASE_URL`.

## Server

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `ISSUER` | yes | | Base URL of the OpenID Connect provider. The server reads its discovery document at startup. |
| `CLIENT_ID` | yes | | Client id registered at the provider. |
| `CLIENT_SECRET` | yes | | Client secret registered at the provider. |
| `COOKIE_SECRET` | yes | | Key that encrypts the session cookie. Use a long random value, and keep it stable across restarts and instances. |
| `APP_URL` | no | `http://localhost:8080` | Public address of the app. The login callback is `<APP_URL>/auth/callback`, so it must match the redirect URI registered at the provider. |
| `DATABASE_URL` | no | unset | `postgres://USER@HOST:PORT/DB` for Postgres, or `sqlite://PATH`. Unset means SQLite at `T_REX_DB_PATH`. |
| `DATABASE_PASSWORD` | no | unset | Postgres password. Never put it in `DATABASE_URL`: the server refuses that. |
| `T_REX_DB_PATH` | no | `~/.t-rex/t-rex.db` | SQLite file, used when `DATABASE_URL` is not set. The folder is created on first start. |
| `RUST_LOG` | no | | Log level, for example `info` or `debug`. |

The server listens on `0.0.0.0:8080`. Put a reverse proxy or an ingress in front of it for TLS.

The server will not start without the OIDC settings: it contacts `ISSUER` at startup. For running it on a laptop, see [development.md](development.md#running-locally).

## Database

- **SQLite** (the default): one file. Fine for one user or a small household, and for a single instance. Keep the file on a persistent disk.
- **Postgres**: set `DATABASE_URL` to the server and database, and `DATABASE_PASSWORD` to the password. The schema is created and migrated at startup, like SQLite.

Changing from one database to the other is not automatic: the data stays in the database it was written to.

Migrations run automatically on every start and are tracked in the `seaql_migrations` table.

## Build

These variables affect `cargo build` and `cargo run`, not the running server.

| Variable | Purpose |
|----------|---------|
| `T_REX_SKIP_WEBAPP` | Set to any value to skip building the web app and reuse the last build. See [development.md](development.md#building). |

## Examples

A minimal `.env` file for a laptop, with SQLite and the default port:

```sh
ISSUER=https://idp.example.com/realms/home
CLIENT_ID=t-rex
CLIENT_SECRET=change-me
COOKIE_SECRET=replace-with-a-long-random-value
APP_URL=http://localhost:8080
RUST_LOG=info
```

A container with Postgres. The password comes from the environment, not from the URL:

```sh
docker run --rm -p 8080:8080 \
  -e ISSUER=https://idp.example.com/realms/home \
  -e CLIENT_ID=t-rex \
  -e CLIENT_SECRET=change-me \
  -e COOKIE_SECRET=replace-with-a-long-random-value \
  -e APP_URL=https://t-rex.example.com \
  -e DATABASE_URL=postgres://t_rex@db.internal:5432/t_rex \
  -e DATABASE_PASSWORD=change-me \
  ghcr.io/aellwein/t-rex:1.0.2
```

A random `COOKIE_SECRET` with 64 hex digits, for example:

```sh
openssl rand -hex 32
```
