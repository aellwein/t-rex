//! Absence kind persistence (vacation, sick leave, ...).

use anyhow::Result;
use dto::{AbsenceKind, AbsenceKindInput, Page};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    QueryFilter, QueryOrder, Set,
    sea_query::{Expr, Func},
};

use crate::entity::absence_kind::{ActiveModel, Column, Entity as Kinds, Model};
use crate::store::{Window, fetch_window, page};

fn from_model(model: Model) -> AbsenceKind {
    AbsenceKind {
        id: model.id,
        name: model.name,
        color: model.color,
        paid: model.paid,
    }
}

/// One window of the absence kinds of `user`, sorted by name, ignoring case.
pub async fn list(
    conn: &DatabaseConnection,
    user: &str,
    window: Window,
) -> Result<Page<AbsenceKind>> {
    let select = Kinds::find()
        .filter(Column::UserId.eq(user))
        .order_by(Expr::expr(Func::lower(Expr::col(Column::Name))), Order::Asc)
        .order_by_asc(Column::Id);
    let (models, total) = fetch_window(select, conn, window).await?;
    Ok(page(
        models.into_iter().map(from_model).collect(),
        total,
        window,
    ))
}

/// One absence kind of `user`, or `None` if it does not exist for that user.
pub async fn get(conn: &DatabaseConnection, user: &str, id: i64) -> Result<Option<AbsenceKind>> {
    let model = Kinds::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?;
    Ok(model.map(from_model))
}

pub async fn create(
    conn: &DatabaseConnection,
    user: &str,
    input: &AbsenceKindInput,
) -> Result<AbsenceKind> {
    let model = ActiveModel {
        user_id: Set(user.to_string()),
        name: Set(input.name.clone()),
        color: Set(input.color.clone()),
        paid: Set(input.paid),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(from_model(model))
}

/// Replaces the fields of an absence kind. Returns `None` if it does not exist for `user`.
pub async fn update(
    conn: &DatabaseConnection,
    user: &str,
    id: i64,
    input: &AbsenceKindInput,
) -> Result<Option<AbsenceKind>> {
    let Some(existing) = Kinds::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    let mut active = existing.into_active_model();
    active.name = Set(input.name.clone());
    active.color = Set(input.color.clone());
    active.paid = Set(input.paid);
    Ok(Some(from_model(active.update(conn).await?)))
}

/// Deletes an absence kind. Returns `false` if it does not exist for `user`.
///
/// Fails with a foreign key error while time entries still reference it.
pub async fn delete(conn: &DatabaseConnection, user: &str, id: i64) -> Result<bool> {
    let result = Kinds::delete_many()
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
    use crate::store::test_db;

    fn input(name: &str, paid: bool) -> AbsenceKindInput {
        AbsenceKindInput {
            name: name.into(),
            color: "#0d9488".into(),
            paid,
        }
    }

    #[tokio::test]
    async fn create_list_and_get_round_trip() {
        let conn = test_db().await;
        let vacation = create(&conn, "alex", &input("Vacation", true))
            .await
            .unwrap();
        create(&conn, "alex", &input("Sick leave", false))
            .await
            .unwrap();

        let names: Vec<String> = list(&conn, "alex", everything())
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|k| k.name)
            .collect();
        assert_eq!(names, vec!["Sick leave", "Vacation"]);
        assert_eq!(
            get(&conn, "alex", vacation.id).await.unwrap(),
            Some(vacation)
        );
    }

    #[tokio::test]
    async fn absence_kinds_are_scoped_to_their_user() {
        let conn = test_db().await;
        let kind = create(&conn, "alex", &input("Vacation", true))
            .await
            .unwrap();
        assert_eq!(get(&conn, "bob", kind.id).await.unwrap(), None);
        assert!(
            list(&conn, "bob", everything())
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }

    #[tokio::test]
    async fn update_and_delete() {
        let conn = test_db().await;
        let kind = create(&conn, "alex", &input("Leave", true)).await.unwrap();

        let updated = update(&conn, "alex", kind.id, &input("Unpaid leave", false))
            .await
            .unwrap()
            .expect("kind exists");
        assert_eq!(updated.name, "Unpaid leave");
        assert!(!updated.paid);

        assert!(delete(&conn, "alex", kind.id).await.unwrap());
        assert!(!delete(&conn, "alex", kind.id).await.unwrap());
    }
}
