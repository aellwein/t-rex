//! Database connection: SQLite (a file) or Postgres, chosen by the configuration. Schema changes are
//! SeaORM migrations in `migration/`.

use std::fs;

use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::config::DatabaseConfig;
use crate::migration::Migrator;

/// Connects to the database the configuration names. A SQLite file and its folder are created if missing.
pub async fn connect_config(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    if let DatabaseConfig::Sqlite(path) = config
        && let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    connect(&config.connection_url())
        .await
        .with_context(|| format!("connecting to database {}", config.describe()))
}

/// Connects to a database URL such as `sqlite::memory:` or `postgres://...`.
pub async fn connect(url: &str) -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(url.to_string());
    // An in-memory database lives per connection, so it must use exactly one.
    if url.contains(":memory:") {
        options.max_connections(1).min_connections(1);
    }
    Ok(Database::connect(options).await?)
}

/// Applies every migration that has not run yet.
pub async fn migrate(conn: &DatabaseConnection) -> Result<()> {
    Migrator::up(conn, None)
        .await
        .context("applying migrations")?;
    Ok(())
}

/// A fresh migrated in-memory database. Keep the connection for the whole test.
#[cfg(test)]
pub(crate) async fn test_db() -> DatabaseConnection {
    let conn = connect("sqlite::memory:")
        .await
        .expect("in-memory database");
    migrate(&conn).await.expect("migrations apply");
    conn
}

#[cfg(test)]
mod tests {
    use std::{env, process};

    use super::*;
    use crate::entity::project;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn migrations_create_a_file_once_and_keep_data_on_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("t-rex-orm-{}-{nanos}", process::id()));
        let path = dir.join("nested").join("t-rex.db");

        {
            let conn = connect_config(&DatabaseConfig::Sqlite(path.clone()))
                .await
                .unwrap();
            migrate(&conn).await.unwrap();
            project::ActiveModel {
                user_id: Set("alex".into()),
                name: Set("Persisted".into()),
                color: Set("#0d9488".into()),
                archived: Set(false),
                ..Default::default()
            }
            .insert(&conn)
            .await
            .unwrap();
        }

        let conn = connect_config(&DatabaseConfig::Sqlite(path.clone()))
            .await
            .unwrap();
        migrate(&conn).await.unwrap();
        assert_eq!(project::Entity::find().all(&conn).await.unwrap().len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod postgres_tests {
    use std::env;

    use super::*;
    use crate::entity::holiday;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    /// Runs against the Postgres in `TEST_POSTGRES_URL` (password included, for example
    /// `postgres://user:pass@localhost:5432/t_rex_test`). The database must be empty: the
    /// test creates the tables and adds a row. Run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn baseline_migrates_and_round_trips_on_postgres() {
        let url = env::var("TEST_POSTGRES_URL").expect("TEST_POSTGRES_URL is set");
        let conn = connect(&url).await.unwrap();
        migrate(&conn).await.unwrap();
        holiday::ActiveModel {
            user_id: Set("tester".into()),
            holiday_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()),
            name: Set("Christmas".into()),
            ..Default::default()
        }
        .insert(&conn)
        .await
        .unwrap();
        assert_eq!(holiday::Entity::find().all(&conn).await.unwrap().len(), 1);
    }
}
