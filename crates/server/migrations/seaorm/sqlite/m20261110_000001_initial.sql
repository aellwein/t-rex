-- Baseline schema of the SeaORM layer for SQLite. The Postgres version is in ../postgres.
CREATE TABLE projects (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id  TEXT    NOT NULL,
    name     TEXT    NOT NULL,
    color    TEXT    NOT NULL DEFAULT '#0d9488',
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    UNIQUE (user_id, name)
);

CREATE TABLE absence_kinds (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT    NOT NULL,
    name    TEXT    NOT NULL,
    color   TEXT    NOT NULL DEFAULT '#0d9488',
    paid    INTEGER NOT NULL DEFAULT 1 CHECK (paid IN (0, 1)),
    UNIQUE (user_id, name)
);

CREATE TABLE time_entries (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         TEXT    NOT NULL,
    entry_date      DATE    NOT NULL,  -- calendar date, stored as YYYY-MM-DD
    minutes         INTEGER NOT NULL CHECK (minutes BETWEEN 1 AND 1440),
    project_id      INTEGER REFERENCES projects (id),
    absence_kind_id INTEGER REFERENCES absence_kinds (id),
    note            TEXT    NOT NULL DEFAULT '',
    start_minute    INTEGER CHECK (start_minute BETWEEN 0 AND 1439),
    break_minutes   INTEGER NOT NULL DEFAULT 0 CHECK (break_minutes BETWEEN 0 AND 1440),
    CHECK ((project_id IS NULL) <> (absence_kind_id IS NULL))
);

CREATE INDEX time_entries_user_date ON time_entries (user_id, entry_date);

CREATE TABLE settings (
    user_id            TEXT    PRIMARY KEY,
    theme              TEXT    NOT NULL DEFAULT 'system',
    language           TEXT    NOT NULL DEFAULT 'system'
                               CHECK (language IN ('system', 'en', 'de', 'ru', 'fr', 'es', 'it', 'pt', 'nl', 'pl', 'zh', 'ja')),
    daily_minutes      INTEGER NOT NULL DEFAULT 480 CHECK (daily_minutes BETWEEN 0 AND 1440),
    show_email         INTEGER NOT NULL DEFAULT 0 CHECK (show_email IN (0, 1)),
    break_minutes      INTEGER NOT NULL DEFAULT 45 CHECK (break_minutes BETWEEN 0 AND 1440),
    carry_over_minutes INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE pending_starts (
    user_id      TEXT    PRIMARY KEY,
    entry_date   DATE    NOT NULL,
    start_minute INTEGER NOT NULL CHECK (start_minute BETWEEN 0 AND 1439)
);

CREATE TABLE holidays (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      TEXT    NOT NULL,
    holiday_date DATE    NOT NULL,
    name         TEXT    NOT NULL,
    UNIQUE (user_id, holiday_date)
);
