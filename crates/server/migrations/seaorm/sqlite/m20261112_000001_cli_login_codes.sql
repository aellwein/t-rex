-- One-time codes of the command line login. Only a SHA-256 hash of each code is stored.
CREATE TABLE cli_login_codes (
    code_hash    TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL,
    email        TEXT,
    display_name TEXT,
    expires_at   INTEGER NOT NULL
);
