# t-rex Helm chart

Runs the t-rex server on Kubernetes. The chart and the image are published with every release:

```sh
helm install t-rex oci://ghcr.io/aellwein/t-rex/t-rex --version 1.2.3 -f my-values.yaml
```

## Modes

- **SQLite** (default): the database is one file on a persistent volume. The app runs as a single replica, and updates replace the pod.
- **Postgres**: set `database.url`. The database is outside the pod, and updates roll without downtime.

## Values

| Key | Default | Purpose |
|-----|---------|---------|
| `oidc.issuer`, `oidc.clientId`, `oidc.clientSecret`, `oidc.cookieSecret`, `oidc.appUrl` | empty | OIDC sign-in. The chart creates a secret from them. |
| `oidc.existingSecret` | empty | Use an existing secret with the keys `ISSUER`, `CLIENT_ID`, `CLIENT_SECRET`, `COOKIE_SECRET` and `APP_URL`. |
| `database.url` | empty | `postgres://USER@HOST:PORT/DB`. Empty means SQLite. |
| `database.password` / `database.existingSecret` | empty | Postgres password, from a value or from a secret with the key `password`. |
| `persistence.size`, `persistence.storageClassName` | `1Gi`, empty | SQLite volume. |
| `httpRoute.enabled`, `httpRoute.parentRefs`, `httpRoute.hostnames` | off | Gateway API route to the app. |
| `image.hash`, `image.tag` | release digest | Image to run. The release sets `hash`. |
| `logLevel` | `info` | `RUST_LOG` of the server. |

The full list is in [values.yaml](values.yaml).

## Secrets

Keep the cookie secret stable across upgrades: changing it signs every user out. Keep the secrets out of the values file in version control, for example with `oidc.existingSecret`.
