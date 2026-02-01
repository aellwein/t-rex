# REST API

All endpoints are under `/api/v1` and exchange JSON. Errors have the shape `{"error": "..."}` with a matching status code.

## Stability

`/api/v1` is a stable contract. Changes within v1 are compatible: new endpoints, new optional request fields and new response fields. Clients must ignore unknown fields. A change that cannot be compatible goes into a new version (`/api/v2`), and v1 keeps working for existing clients.

## Authentication

Requests are authenticated in one of two ways:

- **Browser session** (the web app): the OIDC session cookie set by `/auth`.
- **Token:** `Authorization: Bearer <token>`. Create a token in the web app's Tokens page or with `t-rex login`. See [cli.md](cli.md#sign-in).

Tokens expire 10 days after creation. An account holds at most 15 tokens. Creating a token, and starting the CLI sign-in, need a browser session; a token cannot create tokens.

## Paging

Every list endpoint returns an object, not an array:

```json
{"items": [...], "total": 123, "limit": 50, "offset": 0}
```

- `?limit=N&offset=M` chooses the window. `limit` defaults to 50 and must be between 1 and 200; anything else is `400`.
- `total` counts all matching items. Fetch the next page while `offset + items.len() < total`.
- Time entries keep `from` and `to` as filters and page within them.

Sort order of lists:

| List | Order |
|------|-------|
| projects, absence kinds | name, ignoring case, then id |
| holidays | date, then id |
| time entries | date, then id |
| tokens | newest first |

## Endpoints

| Method | Path | Notes |
|--------|------|-------|
| GET    | `/api/v1/health`                | Liveness, returns `ok` |
| GET    | `/api/v1/version`               | Build version of the server |
| GET    | `/api/v1/me`                    | The signed-in account (`email`, `name`) |
| GET    | `/api/v1/settings`              | Defaults until first saved |
| PUT    | `/api/v1/settings`              | Full settings object: `theme`, `language`, `daily_minutes`, `show_email`, `break_minutes`, `carry_over_minutes` |
| GET    | `/api/v1/projects`              | List, paged |
| POST   | `/api/v1/projects`              | Create (`201`) |
| GET    | `/api/v1/projects/{id}`         | |
| PUT    | `/api/v1/projects/{id}`         | Replace all fields |
| DELETE | `/api/v1/projects/{id}`         | `204`, or `409` while time entries still use it |
| GET    | `/api/v1/absence-kinds`         | Same pattern as projects (`name`, `color`, `paid`) |
| POST   | `/api/v1/absence-kinds`         | Create (`201`) |
| GET    | `/api/v1/absence-kinds/{id}`    | |
| PUT    | `/api/v1/absence-kinds/{id}`    | Replace all fields |
| DELETE | `/api/v1/absence-kinds/{id}`    | `204`, or `409` while time entries still use it |
| GET    | `/api/v1/time-entries`          | Optional `?from=YYYY-MM-DD&to=YYYY-MM-DD` (both inclusive) |
| POST   | `/api/v1/time-entries`          | Exactly one of `project_id` / `absence_kind_id`; rejects overlaps with the same day |
| GET    | `/api/v1/time-entries/{id}`     | |
| PUT    | `/api/v1/time-entries/{id}`     | Replace all fields; same rules as create |
| DELETE | `/api/v1/time-entries/{id}`     | `204` |
| GET    | `/api/v1/pending-start`         | The pending start (`date`, `start_minute`), or `null` |
| PUT    | `/api/v1/pending-start`         | Save the pending start |
| DELETE | `/api/v1/pending-start`         | `204` |
| GET    | `/api/v1/holidays`              | List, paged |
| POST   | `/api/v1/holidays`              | Create (`201`), one holiday per day |
| GET    | `/api/v1/holidays/{id}`         | |
| PUT    | `/api/v1/holidays/{id}`         | |
| DELETE | `/api/v1/holidays/{id}`         | `204` |
| POST   | `/api/v1/holidays/import`       | `{"url": "https://..."}`: adds the all-day events of an `.ics` file |
| POST   | `/api/v1/holidays/import-text`  | `{"text": "BEGIN:VCALENDAR..."}`: the same for pasted content |
| GET    | `/api/v1/tokens`                | Tokens of the account, without their secret. Each has `created_at` and `expires_at` |
| POST   | `/api/v1/tokens`                | Create (`201`). Browser session only. The secret is returned once. `409` when the account holds 15 tokens |
| DELETE | `/api/v1/tokens/{id}`           | `204`, revokes the token |
| GET    | `/api/v1/cli/login`             | Browser sign-in for the CLI: redirects to `127.0.0.1` with a one-time code. Browser session only |
| POST   | `/api/v1/cli/token`             | `{"code": "...", "device": "..."}` exchanges the code for a token (`201`). The code works once |

## Holidays

Imports read `.ics` files and keep only all-day events. Limits:

- a file may be at most 5 MB;
- an address is fetched with a 15 second timeout;
- an event spanning more than 366 days is rejected.

An import reports how many holidays were added. Days that already have a holiday keep their existing entry.

## Settings

| Field | Meaning |
|-------|---------|
| `theme` | `system`, or the id of a colour theme |
| `language` | `system`, `en`, `de`, `fr`, `es`, `it`, `pt`, `nl`, `pl`, `ru`, `zh` or `ja` |
| `daily_minutes` | Expected working time per day, in minutes |
| `show_email` | Show the account's email in the header instead of its name |
| `break_minutes` | Unpaid break per working day, in minutes; new days start with it |
| `carry_over_minutes` | Overtime (or deficit) from before the recorded time, in signed minutes |
