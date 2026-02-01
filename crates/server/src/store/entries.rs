//! Time entry persistence.

use anyhow::Result;
use chrono::NaiveDate;
use dto::{Page, TimeEntry, TimeEntryInput};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set,
    sea_query::{Expr, ExprTrait},
};

use crate::entity::time_entry::{ActiveModel, Column, Entity as Entries, Model};
use crate::store::{Window, fetch_window, page};

fn from_model(model: Model) -> Result<TimeEntry> {
    Ok(TimeEntry {
        id: model.id,
        date: model.entry_date,
        minutes: u32::try_from(model.minutes)?,
        project_id: model.project_id,
        absence_kind_id: model.absence_kind_id,
        note: model.note,
        start_minute: model.start_minute.map(u32::try_from).transpose()?,
        break_minutes: u32::try_from(model.break_minutes)?,
    })
}

/// One window of the entries of `user` whose date lies in `from..=to`, ordered by date.
pub async fn list(
    conn: &DatabaseConnection,
    user: &str,
    from: NaiveDate,
    to: NaiveDate,
    window: Window,
) -> Result<Page<TimeEntry>> {
    let select = Entries::find()
        .filter(Column::UserId.eq(user))
        .filter(Column::EntryDate.gte(from))
        .filter(Column::EntryDate.lte(to))
        .order_by_asc(Column::EntryDate)
        .order_by_asc(Column::Id);
    let (models, total) = fetch_window(select, conn, window).await?;
    let items = models
        .into_iter()
        .map(from_model)
        .collect::<Result<Vec<_>>>()?;
    Ok(page(items, total, window))
}

/// One entry of `user`, or `None` if it does not exist for that user.
pub async fn get(conn: &DatabaseConnection, user: &str, id: i64) -> Result<Option<TimeEntry>> {
    let model = Entries::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?;
    model.map(from_model).transpose()
}

fn active_from_input(user: &str, input: &TimeEntryInput) -> ActiveModel {
    ActiveModel {
        user_id: Set(user.to_string()),
        entry_date: Set(input.date),
        minutes: Set(i64::from(input.minutes)),
        project_id: Set(input.project_id),
        absence_kind_id: Set(input.absence_kind_id),
        note: Set(input.note.clone()),
        start_minute: Set(Some(i64::from(input.start_minute))),
        break_minutes: Set(i64::from(input.break_minutes)),
        ..Default::default()
    }
}

pub async fn create(
    conn: &DatabaseConnection,
    user: &str,
    input: &TimeEntryInput,
) -> Result<TimeEntry> {
    let model = active_from_input(user, input).insert(conn).await?;
    from_model(model)
}

/// Replaces an entry. Returns `None` if it does not exist for `user`.
pub async fn update(
    conn: &DatabaseConnection,
    user: &str,
    id: i64,
    input: &TimeEntryInput,
) -> Result<Option<TimeEntry>> {
    let Some(existing) = Entries::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    let fresh = active_from_input(user, input);
    let mut active = existing.into_active_model();
    active.entry_date = fresh.entry_date;
    active.minutes = fresh.minutes;
    active.project_id = fresh.project_id;
    active.absence_kind_id = fresh.absence_kind_id;
    active.note = fresh.note;
    active.start_minute = fresh.start_minute;
    active.break_minutes = fresh.break_minutes;
    from_model(active.update(conn).await?).map(Some)
}

/// Whether another time record of `user` on the same day overlaps `start..end`.
///
/// Records without a start time (recorded before start times existed) never overlap.
/// `except` names the record being replaced, which is ignored.
pub async fn overlaps(
    conn: &DatabaseConnection,
    user: &str,
    date: NaiveDate,
    start: u32,
    end: u32,
    except: Option<i64>,
) -> Result<bool> {
    let block_end = Expr::col(Column::StartMinute)
        .add(Expr::col(Column::Minutes))
        .add(Expr::col(Column::BreakMinutes));
    let found = Entries::find()
        .filter(Column::UserId.eq(user))
        .filter(Column::EntryDate.eq(date))
        .filter(Column::StartMinute.is_not_null())
        .filter(Column::StartMinute.lt(i64::from(end)))
        .filter(block_end.gt(i64::from(start)))
        .filter(Column::Id.ne(except.unwrap_or(-1)))
        .one(conn)
        .await?;
    Ok(found.is_some())
}

/// Deletes an entry. Returns `false` if it does not exist for `user`.
pub async fn delete(conn: &DatabaseConnection, user: &str, id: i64) -> Result<bool> {
    let result = Entries::delete_many()
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user))
        .exec(conn)
        .await?;
    Ok(result.rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::everything;
    use crate::store::{Table, owned_exists, projects, test_db};
    use dto::ProjectInput;

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
    }

    fn project_entry(date: NaiveDate, project_id: i64, minutes: u32) -> TimeEntryInput {
        TimeEntryInput {
            date,
            start_minute: 0,
            minutes,
            break_minutes: 0,
            project_id: Some(project_id),
            absence_kind_id: None,
            note: "work".into(),
        }
    }

    async fn project(conn: &DatabaseConnection, user: &str, name: &str) -> i64 {
        projects::create(
            conn,
            user,
            &ProjectInput {
                name: name.into(),
                color: "#0d9488".into(),
                archived: false,
            },
        )
        .await
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn create_and_get_a_project_entry() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        let created = create(&conn, "alex", &project_entry(day(5), pid, 90))
            .await
            .unwrap();

        assert_eq!(created.date, day(5));
        assert_eq!(created.minutes, 90);
        assert_eq!(created.project_id, Some(pid));
        assert_eq!(created.absence_kind_id, None);
        assert_eq!(get(&conn, "alex", created.id).await.unwrap(), Some(created));
    }

    #[tokio::test]
    async fn list_includes_both_bounds_and_excludes_outside_dates() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        create(&conn, "alex", &project_entry(day(1), pid, 30))
            .await
            .unwrap();
        create(&conn, "alex", &project_entry(day(15), pid, 30))
            .await
            .unwrap();
        create(&conn, "alex", &project_entry(day(30), pid, 30))
            .await
            .unwrap();
        let december = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        create(&conn, "alex", &project_entry(december, pid, 30))
            .await
            .unwrap();

        let found = list(&conn, "alex", day(1), day(30), everything())
            .await
            .unwrap()
            .items;
        let dates: Vec<NaiveDate> = found.iter().map(|e| e.date).collect();
        assert_eq!(dates, vec![day(1), day(15), day(30)]);
    }

    #[tokio::test]
    async fn entry_with_both_references_violates_the_check_constraint() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        let both = TimeEntryInput {
            absence_kind_id: Some(1),
            ..project_entry(day(5), pid, 30)
        };
        let err = create(&conn, "alex", &both).await.unwrap_err();
        assert!(format!("{err:#}").contains("CHECK constraint failed"));
    }

    #[tokio::test]
    async fn update_changes_the_entry_and_delete_removes_it() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        let entry = create(&conn, "alex", &project_entry(day(5), pid, 30))
            .await
            .unwrap();

        let changed = project_entry(day(6), pid, 45);
        let updated = update(&conn, "alex", entry.id, &changed)
            .await
            .unwrap()
            .expect("entry exists");
        assert_eq!(updated.date, day(6));
        assert_eq!(updated.minutes, 45);

        assert!(delete(&conn, "alex", entry.id).await.unwrap());
        assert_eq!(get(&conn, "alex", entry.id).await.unwrap(), None);
    }

    #[tokio::test]
    async fn entries_of_other_users_are_invisible() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        let entry = create(&conn, "alex", &project_entry(day(5), pid, 30))
            .await
            .unwrap();

        assert_eq!(get(&conn, "bob", entry.id).await.unwrap(), None);
        assert!(
            list(&conn, "bob", day(1), day(30), everything())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(!delete(&conn, "bob", entry.id).await.unwrap());
    }

    #[tokio::test]
    async fn deleting_a_project_with_entries_is_blocked_by_foreign_keys() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        create(&conn, "alex", &project_entry(day(5), pid, 30))
            .await
            .unwrap();

        let err = projects::delete(&conn, "alex", pid).await.unwrap_err();
        assert!(format!("{err:#}").contains("FOREIGN KEY constraint failed"));
    }

    #[tokio::test]
    async fn owned_exists_only_matches_the_owner() {
        let conn = test_db().await;
        let pid = project(&conn, "alex", "Website").await;
        assert!(
            owned_exists(&conn, Table::Projects, pid, "alex")
                .await
                .unwrap()
        );
        assert!(
            !owned_exists(&conn, Table::Projects, pid, "bob")
                .await
                .unwrap()
        );
        assert!(
            !owned_exists(&conn, Table::AbsenceKinds, pid, "alex")
                .await
                .unwrap()
        );
    }
}
