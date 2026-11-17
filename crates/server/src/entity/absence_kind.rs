//! The `absence_kinds` table.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "absence_kinds")]
/// A row of `absence_kinds`: One kind of time off of one user.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: String,
    pub name: String,
    pub color: String,
    pub paid: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No SeaORM relations are declared here. The store matches the ids itself.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
