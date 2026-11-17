//! Holiday persistence.

use anyhow::Result;
use chrono::NaiveDate;
use dto::{Holiday, HolidayInput, Page};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, Set, TryInsertResult,
};

use crate::entity::holiday::{ActiveModel, Column, Entity as Holidays, Model};
use crate::store::{Window, fetch_window, page};

/// Converts a stored row into the DTO.
fn from_model(model: Model) -> Holiday {
    Holiday {
        id: model.id,
        date: model.holiday_date,
        name: model.name,
    }
}

/// All holidays of `user`, sorted by date.
pub async fn list(conn: &DatabaseConnection, user: &str, window: Window) -> Result<Page<Holiday>> {
    let select = Holidays::find()
        .filter(Column::UserId.eq(user))
        .order_by_asc(Column::HolidayDate)
        .order_by_asc(Column::Id);
    let (models, total) = fetch_window(select, conn, window).await?;
    Ok(page(
        models.into_iter().map(from_model).collect(),
        total,
        window,
    ))
}

/// One holiday of `user`, or `None` if it does not exist for that user.
pub async fn get(conn: &DatabaseConnection, user: &str, id: i64) -> Result<Option<Holiday>> {
    let model = Holidays::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?;
    Ok(model.map(from_model))
}

/// The id of the holiday of `user` on `date`, ignoring the holiday `except`.
pub async fn id_on(
    conn: &DatabaseConnection,
    user: &str,
    date: NaiveDate,
    except: Option<i64>,
) -> Result<Option<i64>> {
    let mut query = Holidays::find()
        .filter(Column::UserId.eq(user))
        .filter(Column::HolidayDate.eq(date));
    if let Some(id) = except {
        query = query.filter(Column::Id.ne(id));
    }
    Ok(query.one(conn).await?.map(|model| model.id))
}

/// Inserts a holiday for the user and returns it with its id.
pub async fn create(
    conn: &DatabaseConnection,
    user: &str,
    input: &HolidayInput,
) -> Result<Holiday> {
    let model = ActiveModel {
        user_id: Set(user.to_string()),
        holiday_date: Set(input.date),
        name: Set(input.name.clone()),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(from_model(model))
}

/// Replaces the date and name of a holiday. Returns `None` if it does not exist for `user`.
pub async fn update(
    conn: &DatabaseConnection,
    user: &str,
    id: i64,
    input: &HolidayInput,
) -> Result<Option<Holiday>> {
    let Some(existing) = Holidays::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    let mut active = existing.into_active_model();
    active.holiday_date = Set(input.date);
    active.name = Set(input.name.clone());
    Ok(Some(from_model(active.update(conn).await?)))
}

/// Deletes a holiday. Returns `false` if it does not exist for `user`.
pub async fn delete(conn: &DatabaseConnection, user: &str, id: i64) -> Result<bool> {
    let result = Holidays::delete_many()
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user))
        .exec(conn)
        .await?;
    Ok(result.rows_affected > 0)
}

/// Adds the days that have no holiday yet. Days that already have one keep their name.
/// Returns how many were added and how many were already there.
pub async fn add_missing(
    conn: &DatabaseConnection,
    user: &str,
    days: &[(NaiveDate, String)],
) -> Result<(u32, u32)> {
    let mut added = 0u32;
    for (date, name) in days {
        let model = ActiveModel {
            user_id: Set(user.to_string()),
            holiday_date: Set(*date),
            name: Set(name.clone()),
            ..Default::default()
        };
        let inserted = Holidays::insert(model)
            .on_conflict_do_nothing_on([Column::UserId, Column::HolidayDate])
            .exec_without_returning(conn)
            .await?;
        if let TryInsertResult::Inserted(rows) = inserted {
            added += rows as u32;
        }
    }
    let existing = days.len() as u32 - added;
    Ok((added, existing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::everything;
    use crate::store::test_db;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 12, day).unwrap()
    }

    fn input(day: u32, name: &str) -> HolidayInput {
        HolidayInput {
            date: date(day),
            name: name.into(),
        }
    }

    #[tokio::test]
    async fn create_list_update_and_delete() {
        let conn = test_db().await;
        let christmas = create(&conn, "alex", &input(25, "Christmas"))
            .await
            .unwrap();
        create(&conn, "alex", &input(24, "Christmas Eve"))
            .await
            .unwrap();

        let days: Vec<NaiveDate> = list(&conn, "alex", everything())
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|h| h.date)
            .collect();
        assert_eq!(days, vec![date(24), date(25)]);

        let moved = update(&conn, "alex", christmas.id, &input(26, "Second day"))
            .await
            .unwrap()
            .expect("holiday exists");
        assert_eq!(moved.date, date(26));
        assert!(delete(&conn, "alex", christmas.id).await.unwrap());
        assert!(get(&conn, "alex", christmas.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn holidays_are_scoped_to_their_user() {
        let conn = test_db().await;
        let holiday = create(&conn, "alex", &input(25, "Christmas"))
            .await
            .unwrap();
        assert!(get(&conn, "bob", holiday.id).await.unwrap().is_none());
        assert!(
            list(&conn, "bob", everything())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        // The same day can be a holiday for another user.
        assert!(create(&conn, "bob", &input(25, "Christmas")).await.is_ok());
    }

    #[tokio::test]
    async fn one_holiday_per_day_per_user() {
        let conn = test_db().await;
        create(&conn, "alex", &input(25, "Christmas"))
            .await
            .unwrap();
        assert!(create(&conn, "alex", &input(25, "Again")).await.is_err());
        assert!(
            id_on(&conn, "alex", date(25), None)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn import_keeps_existing_days_and_counts_them() {
        let conn = test_db().await;
        create(&conn, "alex", &input(25, "My name")).await.unwrap();
        let days = vec![
            (date(25), "Christmas".to_string()),
            (date(26), "Boxing Day".to_string()),
        ];
        assert_eq!(add_missing(&conn, "alex", &days).await.unwrap(), (1, 1));

        let all = list(&conn, "alex", everything()).await.unwrap().items;
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "My name");
    }
}
