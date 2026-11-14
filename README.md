# 🦖 t-rex - Time Recordings

Track the time you spend on projects and your time off (absences), day by day. A responsive web app, a command line client and a REST API share one server.

## Layout

| Path          | What it is |
|---------------|------------|
| `crates/dto/`        | Data transfer objects and calendar rules, shared by server and web app (no server dependencies). |
| `crates/server/`     | Axum REST API under `/api/v1`, SQLite or Postgres through SeaORM, OIDC login, serves the web app. |
| `crates/webapp/`     | Dioxus WASM client (single page app, responsive). |
| `crates/cli/`        | Command line client `t-rex` (clap), talks to the REST API. |
| `chart/`      | Helm chart to run the server on Kubernetes. |
| `docs/`       | Documentation, see below. |

## Quick start

You need Rust, the WASM target and the Dioxus CLI, and an OpenID Connect provider to sign in with. See [development.md](docs/development.md).

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli
ISSUER=... CLIENT_ID=... CLIENT_SECRET=... COOKIE_SECRET=... cargo run -p server
```

Then open `http://localhost:8080`.

## Helm chart

Each release publishes the chart and the image. Install the release with:

```sh
helm install t-rex oci://ghcr.io/aellwein/t-rex/t-rex --version 1.2.3 \
  --set oidc.issuer=https://idp.example.com \
  --set oidc.clientId=... --set oidc.clientSecret=... \
  --set oidc.cookieSecret=... --set oidc.appUrl=https://t-rex.example.com
```

See [chart/README.md](chart/README.md) for the values. Use `oidc.existingSecret` and `database.existingSecret` to keep secrets out of the command line.

## Documentation

- [Architecture](docs/architecture.md): how the crates fit together, authentication, storage and the domain rules.
- [REST API](docs/api.md): endpoints, paging and the stable `/api/v1` contract.
- [Command line](docs/cli.md): the `t-rex` client, sign-in, tokens and shell completion.
- [Configuration](docs/configuration.md): environment variables of the server.
- [Development](docs/development.md): building, tests, formatting and changing the API or schema.

## License

MIT. See [LICENSE](LICENSE).
