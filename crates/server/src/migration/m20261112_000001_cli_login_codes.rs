//! One-time codes of the command line login. The SQL is per database, like the baseline.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
/// Adds the `cli_login_codes` table for the one-time codes of the command line login.
pub struct Migration;

const SQLITE_UP: &str =
    include_str!("../../migrations/seaorm/sqlite/m20261112_000001_cli_login_codes.sql");
const POSTGRES_UP: &str =
    include_str!("../../migrations/seaorm/postgres/m20261112_000001_cli_login_codes.sql");

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = match manager.get_database_backend() {
            DatabaseBackend::Sqlite => SQLITE_UP,
            DatabaseBackend::Postgres => POSTGRES_UP,
            other => return Err(DbErr::Custom(format!("no schema for {other:?}"))),
        };
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE cli_login_codes;")
            .await?;
        Ok(())
    }
}
