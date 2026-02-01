//! The `api_tokens` table: one row per command line token. Only the hash of the token is stored.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: String,
    pub name: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub token_hash: String,
    pub created_at: String,
    /// Unix time in seconds after which the token is refused.
    pub expires_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
