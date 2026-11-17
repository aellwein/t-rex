//! Project persistence.

use anyhow::Result;
use dto::{Page, Project, ProjectInput};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Order,
    QueryFilter, QueryOrder, Set,
    sea_query::{Expr, Func},
};

use crate::entity::project::{ActiveModel, Column, Entity as Projects, Model};
use crate::store::{Window, fetch_window, page};

/// Converts a stored row into the DTO.
fn from_model(model: Model) -> Project {
    Project {
        id: model.id,
        name: model.name,
        color: model.color,
        archived: model.archived,
    }
}

/// One window of the projects of `user`, sorted by name, ignoring case.
pub async fn list(conn: &DatabaseConnection, user: &str, window: Window) -> Result<Page<Project>> {
    let select = Projects::find()
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

/// One project of `user`, or `None` if it does not exist for that user.
pub async fn get(conn: &DatabaseConnection, user: &str, id: i64) -> Result<Option<Project>> {
    let model = Projects::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?;
    Ok(model.map(from_model))
}

/// Inserts a project for the user and returns it with its id. A duplicate name fails with a unique-constraint error, which the API answers with `409`.
pub async fn create(
    conn: &DatabaseConnection,
    user: &str,
    input: &ProjectInput,
) -> Result<Project> {
    let model = ActiveModel {
        user_id: Set(user.to_string()),
        name: Set(input.name.clone()),
        color: Set(input.color.clone()),
        archived: Set(input.archived),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(from_model(model))
}

/// Replaces the fields of a project. Returns `None` if it does not exist for `user`.
pub async fn update(
    conn: &DatabaseConnection,
    user: &str,
    id: i64,
    input: &ProjectInput,
) -> Result<Option<Project>> {
    let Some(existing) = Projects::find_by_id(id)
        .filter(Column::UserId.eq(user))
        .one(conn)
        .await?
    else {
        return Ok(None);
    };
    let mut active = existing.into_active_model();
    active.name = Set(input.name.clone());
    active.color = Set(input.color.clone());
    active.archived = Set(input.archived);
    Ok(Some(from_model(active.update(conn).await?)))
}

/// Deletes a project. Returns `false` if it does not exist for `user`.
///
/// Fails with a foreign key error while time entries still reference the project.
pub async fn delete(conn: &DatabaseConnection, user: &str, id: i64) -> Result<bool> {
    let result = Projects::delete_many()
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

    fn input(name: &str) -> ProjectInput {
        ProjectInput {
            name: name.into(),
            color: "#0d9488".into(),
            archived: false,
        }
    }

    #[tokio::test]
    async fn create_then_get_returns_the_same_project() {
        let conn = test_db().await;
        let created = create(&conn, "alex", &input("Website")).await.unwrap();
        let fetched = get(&conn, "alex", created.id).await.unwrap();
        assert_eq!(fetched, Some(created));
    }

    #[tokio::test]
    async fn list_is_sorted_by_name_and_only_shows_the_users_projects() {
        let conn = test_db().await;
        create(&conn, "alex", &input("beta")).await.unwrap();
        create(&conn, "alex", &input("Alpha")).await.unwrap();
        create(&conn, "bob", &input("Gamma")).await.unwrap();

        let names: Vec<String> = list(&conn, "alex", everything())
            .await
            .unwrap()
            .items
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["Alpha", "beta"]);
    }

    #[tokio::test]
    async fn other_users_cannot_read_change_or_delete_a_project() {
        let conn = test_db().await;
        let project = create(&conn, "alex", &input("Secret")).await.unwrap();

        assert_eq!(get(&conn, "bob", project.id).await.unwrap(), None);
        assert_eq!(
            update(&conn, "bob", project.id, &input("Hijacked"))
                .await
                .unwrap(),
            None
        );
        assert!(!delete(&conn, "bob", project.id).await.unwrap());
        assert!(get(&conn, "alex", project.id).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn names_are_unique_per_user_but_not_across_users() {
        let conn = test_db().await;
        create(&conn, "alex", &input("Dup")).await.unwrap();

        let err = create(&conn, "alex", &input("Dup")).await.unwrap_err();
        assert!(format!("{err:#}").contains("UNIQUE constraint failed"));

        create(&conn, "bob", &input("Dup")).await.unwrap();
    }

    #[tokio::test]
    async fn update_replaces_the_fields() {
        let conn = test_db().await;
        let project = create(&conn, "alex", &input("Old")).await.unwrap();
        let changed = ProjectInput {
            name: "New".into(),
            color: "#ff0000".into(),
            archived: true,
        };

        let updated = update(&conn, "alex", project.id, &changed)
            .await
            .unwrap()
            .expect("project exists");
        assert_eq!(updated.id, project.id);
        assert_eq!(updated.name, "New");
        assert_eq!(updated.color, "#ff0000");
        assert!(updated.archived);
    }

    #[tokio::test]
    async fn update_of_missing_project_returns_none() {
        let conn = test_db().await;
        assert_eq!(
            update(&conn, "alex", 999, &input("Ghost")).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn delete_removes_the_project_and_reports_missing_ones() {
        let conn = test_db().await;
        let project = create(&conn, "alex", &input("Temp")).await.unwrap();

        assert!(delete(&conn, "alex", project.id).await.unwrap());
        assert_eq!(get(&conn, "alex", project.id).await.unwrap(), None);
        assert!(!delete(&conn, "alex", project.id).await.unwrap());
    }
}
