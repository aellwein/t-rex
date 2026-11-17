//! Baseline schema. Every table is scoped by `user_id`.
//!
//! The SQL is kept as text on purpose: CHECK constraints and composite keys are easier to
//! review this way than in the query builder. One file per database: `migrations/seaorm/`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
/// The first schema: projects, absence kinds, time entries, settings, pending starts and holidays.
pub struct Migration;

/// The statements that create the first schema on SQLite, read from the migration folder.
const SQLITE_UP: &str = include_str!("../../migrations/seaorm/sqlite/m20261110_000001_initial.sql");
const POSTGRES_UP: &str =
    include_str!("../../migrations/seaorm/postgres/m20261110_000001_initial.sql");

/// The statements that drop the first schema again.
const DOWN: &str = r#"
DROP TABLE holidays;
DROP TABLE pending_starts;
DROP TABLE settings;
DROP INDEX time_entries_user_date;
DROP TABLE time_entries;
DROP TABLE absence_kinds;
DROP TABLE projects;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = match manager.get_database_backend() {
            DatabaseBackend::Sqlite => SQLITE_UP,
            DatabaseBackend::Postgres => POSTGRES_UP,
            other => return Err(DbErr::Custom(format!("no baseline schema for {other:?}"))),
        };
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
