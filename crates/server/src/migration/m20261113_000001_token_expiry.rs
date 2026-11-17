//! Tokens expire. The column is added for both databases; tokens that already exist expire ten days
//! after their creation, the lifetime that applies from now on.

use chrono::DateTime;
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
/// Adds the expiry of tokens. Tokens that exist already expire ten days after this migration.
pub struct Migration;

/// Ten days in seconds, the lifetime of a token at the time of this migration.
const LIFETIME_SECS: i64 = 10 * 24 * 60 * 60;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            "ALTER TABLE api_tokens ADD COLUMN expires_at BIGINT NOT NULL DEFAULT 0;",
        )
        .await?;

        let backend = manager.get_database_backend();
        let rows = conn
            .query_all_raw(Statement::from_string(
                backend,
                "SELECT id, created_at FROM api_tokens".to_string(),
            ))
            .await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let created: String = row.try_get("", "created_at")?;
            let created = DateTime::parse_from_rfc3339(&created)
                .map_err(|err| DbErr::Custom(format!("token {id} has a bad created_at: {err}")))?;
            // Only integers go into the statement, so no quoting is needed.
            let expires = created.timestamp() + LIFETIME_SECS;
            conn.execute_unprepared(&format!(
                "UPDATE api_tokens SET expires_at = {expires} WHERE id = {id};"
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE api_tokens DROP COLUMN expires_at;")
            .await?;
        Ok(())
    }
}
