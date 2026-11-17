//! The `holidays` table: days without working time, unique per user and day.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "holidays")]
/// A row of `holidays`: One day without working time of one user.
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: String,
    pub holiday_date: Date,
    pub name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// No SeaORM relations are declared here. The store matches the ids itself.
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
