//! Persistence layer: SeaORM queries for each entity. Every query is scoped by user id.

/// Queries for absence kinds.
pub mod absences;
/// Queries for API tokens: creation within the per-account limit, lookup by hash, listing and revoking.
pub mod api_tokens;
/// Queries for the one-time codes of the command line login.
pub mod cli_logins;
/// Queries for time records, including the overlap check.
pub mod entries;
/// Queries for holidays, including the bulk import from a calendar.
pub mod holidays;
/// Queries for the pending start.
pub mod pending;
/// Queries for projects.
pub mod projects;
/// Queries for the settings.
pub mod settings;

use anyhow::Result;
use dto::Page;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QuerySelect, Select,
};

use crate::entity::{absence_kind, project};

#[cfg(test)]
pub(crate) use crate::db::test_db;

/// The part of a list one request asks for: `limit` items starting at `offset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub limit: u64,
    pub offset: u64,
}

/// Runs `select` for one window. Returns the rows of the window and the number of rows in total.
/// Callers add their ordering to `select` first, with a unique column last, so pages do not overlap.
pub async fn fetch_window<E>(
    select: Select<E>,
    conn: &DatabaseConnection,
    window: Window,
) -> Result<(Vec<E::Model>, u64)>
where
    E: EntityTrait,
    E::Model: FromQueryResult + Send + Sync,
{
    let total = PaginatorTrait::count(select.clone(), conn).await?;
    let items = select
        .limit(window.limit)
        .offset(window.offset)
        .all(conn)
        .await?;
    Ok((items, total))
}

/// Wraps the rows of one window, converted to their DTO form, into a [`Page`].
pub fn page<T>(items: Vec<T>, total: u64, window: Window) -> Page<T> {
    Page {
        items,
        total,
        limit: window.limit,
        offset: window.offset,
    }
}

/// Tables that time entries reference and that ownership checks run against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Projects,
    AbsenceKinds,
}

/// Whether a row with `id` exists in `table` and belongs to `user`.
pub async fn owned_exists(
    conn: &DatabaseConnection,
    table: Table,
    id: i64,
    user: &str,
) -> Result<bool> {
    let found = match table {
        Table::Projects => project::Entity::find_by_id(id)
            .filter(project::Column::UserId.eq(user))
            .one(conn)
            .await?
            .is_some(),
        Table::AbsenceKinds => absence_kind::Entity::find_by_id(id)
            .filter(absence_kind::Column::UserId.eq(user))
            .one(conn)
            .await?
            .is_some(),
    };
    Ok(found)
}

/// A window that holds every row a test creates.
#[cfg(test)]
pub(crate) fn everything() -> Window {
    Window {
        limit: dto::MAX_LIMIT,
        offset: 0,
    }
}
