-- Baseline schema of the SeaORM layer for Postgres. Same tables and rules as the SQLite version.
-- Integers are BIGINT because the Rust side reads them as i64; booleans are real BOOLEAN columns.

CREATE TABLE projects (
    id       BIGSERIAL PRIMARY KEY,
    user_id  TEXT    NOT NULL,
    name     TEXT    NOT NULL,
    color    TEXT    NOT NULL DEFAULT '#0d9488',
    archived BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (user_id, name)
);

CREATE TABLE absence_kinds (
    id      BIGSERIAL PRIMARY KEY,
    user_id TEXT    NOT NULL,
    name    TEXT    NOT NULL,
    color   TEXT    NOT NULL DEFAULT '#0d9488',
    paid    BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE (user_id, name)
);

CREATE TABLE time_entries (
    id              BIGSERIAL PRIMARY KEY,
    user_id         TEXT   NOT NULL,
    entry_date      DATE   NOT NULL,
    minutes         BIGINT NOT NULL CHECK (minutes BETWEEN 1 AND 1440),
    project_id      BIGINT REFERENCES projects (id),
    absence_kind_id BIGINT REFERENCES absence_kinds (id),
    note            TEXT   NOT NULL DEFAULT '',
    start_minute    BIGINT CHECK (start_minute BETWEEN 0 AND 1439),
    break_minutes   BIGINT NOT NULL DEFAULT 0 CHECK (break_minutes BETWEEN 0 AND 1440),
    CHECK ((project_id IS NULL) <> (absence_kind_id IS NULL))
);

CREATE INDEX time_entries_user_date ON time_entries (user_id, entry_date);

CREATE TABLE settings (
    user_id            TEXT    PRIMARY KEY,
    theme              TEXT    NOT NULL DEFAULT 'system',
    language           TEXT    NOT NULL DEFAULT 'system'
                               CHECK (language IN ('system', 'en', 'de', 'ru', 'fr', 'es', 'it', 'pt', 'nl', 'pl', 'zh', 'ja')),
    daily_minutes      BIGINT  NOT NULL DEFAULT 480 CHECK (daily_minutes BETWEEN 0 AND 1440),
    show_email         BOOLEAN NOT NULL DEFAULT FALSE,
    break_minutes      BIGINT  NOT NULL DEFAULT 45 CHECK (break_minutes BETWEEN 0 AND 1440),
    carry_over_minutes BIGINT  NOT NULL DEFAULT 0
);

CREATE TABLE pending_starts (
    user_id      TEXT   PRIMARY KEY,
    entry_date   DATE   NOT NULL,
    start_minute BIGINT NOT NULL CHECK (start_minute BETWEEN 0 AND 1439)
);

CREATE TABLE holidays (
    id           BIGSERIAL PRIMARY KEY,
    user_id      TEXT NOT NULL,
    holiday_date DATE NOT NULL,
    name         TEXT NOT NULL,
    UNIQUE (user_id, holiday_date)
);
