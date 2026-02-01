-- API tokens for the command line client. Only a SHA-256 hash of each token is stored.
CREATE TABLE api_tokens (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    TEXT NOT NULL,
    name       TEXT NOT NULL,
    email      TEXT,
    display_name TEXT,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE INDEX api_tokens_user ON api_tokens (user_id);
