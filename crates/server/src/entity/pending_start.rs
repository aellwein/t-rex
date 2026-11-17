//! The `pending_starts` table: work that has started but has no time record yet. One per account.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "pending_starts")]
/// A row of `pending_starts`: The start of work that has no time record yet, one per user.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub entry_date: Date,
    pub start_minute: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No SeaORM relations are declared here. The store matches the ids itself.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
