//! The start of unfinished work per account.

use anyhow::Result;
use dto::PendingStart;
use sea_orm::{DatabaseConnection, EntityTrait, Set, sea_query::OnConflict};

use crate::entity::pending_start::{ActiveModel, Column, Entity as Pending, Model};

/// Converts a stored row into the DTO. Fails if a stored number does not fit its type.
fn from_model(model: Model) -> Result<PendingStart> {
    Ok(PendingStart {
        date: model.entry_date,
        start_minute: u32::try_from(model.start_minute)?,
    })
}

/// The pending start of `user`, if there is one.
pub async fn get(conn: &DatabaseConnection, user: &str) -> Result<Option<PendingStart>> {
    match Pending::find_by_id(user.to_string()).one(conn).await? {
        Some(model) => Ok(Some(from_model(model)?)),
        None => Ok(None),
    }
}

/// Saves the pending start of `user`, replacing any earlier one.
pub async fn put(
    conn: &DatabaseConnection,
    user: &str,
    pending: &PendingStart,
) -> Result<PendingStart> {
    let model = ActiveModel {
        user_id: Set(user.to_string()),
        entry_date: Set(pending.date),
        start_minute: Set(i64::from(pending.start_minute)),
    };
    Pending::insert(model)
        .on_conflict(
            OnConflict::column(Column::UserId)
                .update_columns([Column::EntryDate, Column::StartMinute])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    Ok(*pending)
}

/// Removes the pending start of `user`. Returns `false` if there was none.
pub async fn delete(conn: &DatabaseConnection, user: &str) -> Result<bool> {
    let result = Pending::delete_by_id(user.to_string()).exec(conn).await?;
    Ok(result.rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_db;
    use chrono::NaiveDate;

    fn pending(day: u32, start_minute: u32) -> PendingStart {
        PendingStart {
            date: NaiveDate::from_ymd_opt(2026, 11, day).unwrap(),
            start_minute,
        }
    }

    #[tokio::test]
    async fn put_replaces_and_delete_clears() {
        let conn = test_db().await;
        assert_eq!(get(&conn, "alex").await.unwrap(), None);
        put(&conn, "alex", &pending(6, 480)).await.unwrap();
        put(&conn, "alex", &pending(6, 495)).await.unwrap();
        assert_eq!(get(&conn, "alex").await.unwrap(), Some(pending(6, 495)));
        assert!(delete(&conn, "alex").await.unwrap());
        assert_eq!(get(&conn, "alex").await.unwrap(), None);
        assert!(!delete(&conn, "alex").await.unwrap());
    }

    #[tokio::test]
    async fn pending_starts_are_per_account() {
        let conn = test_db().await;
        put(&conn, "alex", &pending(6, 480)).await.unwrap();
        assert_eq!(get(&conn, "sam").await.unwrap(), None);
    }
}
