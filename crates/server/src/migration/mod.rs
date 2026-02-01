//! Schema migrations. Forward only: add new ones at the end of `migrations()` with the next name.
//! `m20261110_000001_initial` is the baseline of the SeaORM layer; the old libsql history is not replayed.

mod m20261110_000001_initial;
mod m20261111_000001_api_tokens;
mod m20261112_000001_cli_login_codes;
mod m20261113_000001_token_expiry;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261110_000001_initial::Migration),
            Box::new(m20261111_000001_api_tokens::Migration),
            Box::new(m20261112_000001_cli_login_codes::Migration),
            Box::new(m20261113_000001_token_expiry::Migration),
        ]
    }
}
