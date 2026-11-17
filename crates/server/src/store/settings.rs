//! Per-user settings persistence.

use anyhow::{Result, anyhow};
use dto::{Language, Settings};
use sea_orm::{DatabaseConnection, EntityTrait, Set, sea_query::OnConflict};

use crate::entity::setting::{ActiveModel, Column, Entity as Settings_, Model};

/// Converts a stored row into the DTO. Fails if a stored value does not fit its type.
fn from_model(model: Model) -> Result<Settings> {
    let language = Language::parse(&model.language)
        .ok_or_else(|| anyhow!("unknown language '{}' in database", model.language))?;
    Ok(Settings {
        theme: model.theme,
        language,
        daily_minutes: u32::try_from(model.daily_minutes)?,
        show_email: model.show_email,
        break_minutes: u32::try_from(model.break_minutes)?,
        carry_over_minutes: i32::try_from(model.carry_over_minutes)?,
    })
}

/// The saved settings of `user`, or the defaults if none were saved yet.
pub async fn get(conn: &DatabaseConnection, user: &str) -> Result<Settings> {
    match Settings_::find_by_id(user.to_string()).one(conn).await? {
        Some(model) => from_model(model),
        None => Ok(Settings::default()),
    }
}

/// Saves the settings of `user` (insert or update) and returns the stored values.
pub async fn put(conn: &DatabaseConnection, user: &str, settings: &Settings) -> Result<Settings> {
    let model = ActiveModel {
        user_id: Set(user.to_string()),
        theme: Set(settings.theme.clone()),
        language: Set(settings.language.as_str().to_string()),
        daily_minutes: Set(i64::from(settings.daily_minutes)),
        show_email: Set(settings.show_email),
        break_minutes: Set(i64::from(settings.break_minutes)),
        carry_over_minutes: Set(i64::from(settings.carry_over_minutes)),
    };
    Settings_::insert(model)
        .on_conflict(
            OnConflict::column(Column::UserId)
                .update_columns([
                    Column::Theme,
                    Column::Language,
                    Column::DailyMinutes,
                    Column::ShowEmail,
                    Column::BreakMinutes,
                    Column::CarryOverMinutes,
                ])
                .to_owned(),
        )
        .exec_without_returning(conn)
        .await?;
    get(conn, user).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::test_db;

    #[tokio::test]
    async fn defaults_are_returned_until_something_is_saved() {
        let conn = test_db().await;
        assert_eq!(get(&conn, "alex").await.unwrap(), Settings::default());
    }

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let conn = test_db().await;
        let settings = Settings {
            theme: "nord-dark".into(),
            language: Language::Ru,
            daily_minutes: 450,
            show_email: true,
            break_minutes: 45,
            carry_over_minutes: 0,
        };
        assert_eq!(put(&conn, "alex", &settings).await.unwrap(), settings);
        assert_eq!(get(&conn, "alex").await.unwrap(), settings);
    }

    #[tokio::test]
    async fn put_overwrites_the_existing_row() {
        let conn = test_db().await;
        put(&conn, "alex", &Settings::default()).await.unwrap();

        let changed = Settings {
            theme: "nord-dark".into(),
            language: Language::De,
            daily_minutes: 300,
            show_email: false,
            break_minutes: 0,
            carry_over_minutes: -90,
        };
        put(&conn, "alex", &changed).await.unwrap();
        assert_eq!(get(&conn, "alex").await.unwrap(), changed);
    }

    #[tokio::test]
    async fn settings_are_per_user() {
        let conn = test_db().await;
        let dark = Settings {
            theme: "nord-dark".into(),
            ..Settings::default()
        };
        put(&conn, "alex", &dark).await.unwrap();
        assert_eq!(get(&conn, "bob").await.unwrap(), Settings::default());
    }
}
