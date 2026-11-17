//! The `settings` table: one row per account.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "settings")]
/// A row of `settings`: The settings of one user, one row per user.
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub theme: String,
    pub language: String,
    pub daily_minutes: i64,
    pub show_email: bool,
    pub break_minutes: i64,
    pub carry_over_minutes: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No SeaORM relations are declared here. The store matches the ids itself.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
