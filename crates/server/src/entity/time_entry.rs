//! The `time_entries` table: one work or absence record per row.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "time_entries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub user_id: String,
    pub entry_date: Date,
    pub minutes: i64,
    pub project_id: Option<i64>,
    pub absence_kind_id: Option<i64>,
    pub note: String,
    pub start_minute: Option<i64>,
    pub break_minutes: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
