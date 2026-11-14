# Development

## Requirements

- Rust (stable, edition 2021 and 2024 are both used in the workspace)
- The WASM target for the web app: `rustup target add wasm32-unknown-unknown`
- The Dioxus CLI: `cargo install dioxus-cli`. The server build runs it to build the web app.
- An OpenID Connect provider for signing in (see [running locally](#running-locally))

## Building

```sh
cargo run -p server        # builds and embeds the web app, then starts the server
```

Building the server also builds the web app, so a server build needs `dx` and the WASM target. The web app is rebuilt whenever its sources (`crates/webapp/`, `crates/dto/`) change. To reuse the last build, set `T_REX_SKIP_WEBAPP=1`:

```sh
T_REX_SKIP_WEBAPP=1 cargo run -p server
```

## Tests

```sh
cargo test -p dto                            # DTO and calendar rules
cargo test -p server --no-default-features   # API, store and database tests; no web app build
cargo test -p t-rex-cli                      # CLI unit tests
```

The server tests run against an in-memory SQLite database. Two tests need a real Postgres:

```sh
TEST_POSTGRES_URL=postgres://user:pass@localhost:5432/t_rex_test cargo test -p server -- --ignored
```

The database must be empty: the test creates the tables and adds a row.

## Formatting and lints

The workspace has `rustfmt.toml` (import and module ordering). Before committing:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
```

Keep clippy at zero warnings.

## Running locally

The server needs an OpenID Connect provider, because it reads the provider's configuration at startup and signs users in through it. Set `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`, `COOKIE_SECRET` and `APP_URL` (see [configuration.md](configuration.md)). Register `http://localhost:8080/auth/callback` as the redirect URI at the provider.

Without `DATABASE_URL` the database is a SQLite file at `~/.t-rex/t-rex.db`, created on the first start. Set `T_REX_DB_PATH` to use another file.

Then open `http://localhost:8080`.

## Working on the CLI

The CLI talks to a running server. To try it against a local server:

```sh
cargo run -p t-rex-cli -- --server http://localhost:8080 login
```

When an endpoint or a setting changes, update the CLI too: `crates/cli/src/cli.rs` (the commands) and `crates/cli/src/commands.rs` (the calls). The shared `dto` types make most changes show up as compile errors.

## Changing the API

`/api/v1` is a stable contract (see [api.md](api.md#stability)). Additive changes are fine. Anything that breaks existing clients needs a new version.

When you add a list endpoint, page it with `PageQuery` and return `Page<T>`, like the existing lists.

## Changing the schema

1. Add a migration in `crates/server/src/migration/`, registered in `migration/mod.rs`.
2. Keep it compatible with the previous app version: a rolling update runs both at once.
3. Write the SQL for both SQLite and Postgres, as the baseline does.
4. Run the tests on SQLite, and the ignored Postgres test if the change touches Postgres-specific behaviour.

## Releasing

A release is a tag. Use a SemVer version without a `v` prefix, because Helm chart versions must be SemVer:

```sh
git tag 1.2.3
git push origin 1.2.3
```

The `Release` workflow (`.github/workflows/release.yml`) then:

1. builds the image for `linux/amd64` and `linux/arm64`, each on a runner of its own architecture;
2. publishes one multi-arch image as `ghcr.io/aellwein/t-rex:1.2.3`;
3. sets the chart version and app version to the tag, and the image to the image digest;
4. pushes the chart to `oci://ghcr.io/aellwein/t-rex/t-rex:1.2.3`, signed with GPG.

The workflow needs these repository secrets: `GPG_PRIVATE_KEY` and `GPG_PASSPHRASE`. The registry is written with the built-in `GITHUB_TOKEN`. The first release creates the packages; set them to public in the package settings if others should pull them without signing in.

A tag that is not SemVer fails the workflow before anything is built.
