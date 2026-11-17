//! The `cli_login_codes` table: one row per unused login code of the command line client.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "cli_login_codes")]
/// A row of `cli_login_codes`: A pending command line login. Only the SHA-256 hash of the one-time code is kept.
pub struct Model {
    /// SHA-256 hash of the code. The code itself is never stored.
    #[sea_orm(primary_key, auto_increment = false)]
    pub code_hash: String,
    pub user_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// Unix time in seconds after which the code is no longer accepted.
    pub expires_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No SeaORM relations are declared here. The store matches the ids itself.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
