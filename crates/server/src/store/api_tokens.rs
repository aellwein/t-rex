//! API token persistence. Tokens are looked up by hash; the secret value is never stored.

use anyhow::Result;
use chrono::{DateTime, SecondsFormat};
use dto::{Page, TokenInfo};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, Statement, TransactionTrait,
};

use crate::entity::api_token::{ActiveModel, Column, Entity as Tokens, Model};
use crate::store::{Window, fetch_window, page};

/// How long a token stays valid after it is created, in seconds (10 days).
pub const TOKEN_LIFETIME_SECS: i64 = 10 * 24 * 60 * 60;

/// Most tokens one account may hold at a time. Expired tokens do not count once they are removed.
pub const MAX_TOKENS_PER_USER: u64 = 15;

/// Removes the tokens of `user` that have expired by `now` (Unix seconds). Returns how many were removed.
pub async fn delete_expired<C: ConnectionTrait>(conn: &C, user: &str, now: i64) -> Result<u64> {
    let result = Tokens::delete_many()
        .filter(Column::UserId.eq(user))
        .filter(Column::ExpiresAt.lte(now))
        .exec(conn)
        .await?;
    Ok(result.rows_affected)
}

/// How many tokens `user` holds, including expired ones that have not been removed yet.
pub async fn count<C: ConnectionTrait>(conn: &C, user: &str) -> Result<u64> {
    let select = Tokens::find().filter(Column::UserId.eq(user));
    Ok(PaginatorTrait::count(select, conn).await?)
}

/// The account a token belongs to, as stored with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenOwner {
    pub user_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

/// Everything needed to store a new token.
pub struct NewToken<'a> {
    pub user_id: &'a str,
    pub name: &'a str,
    pub email: Option<&'a str>,
    pub display_name: Option<&'a str>,
    pub token_hash: &'a str,
    pub created_at: &'a str,
    /// Unix time in seconds after which the token is refused.
    pub expires_at: i64,
}

/// Formats Unix seconds as an RFC 3339 timestamp in UTC.
fn rfc3339(seconds: i64) -> String {
    DateTime::from_timestamp(seconds, 0)
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

fn info(model: &Model) -> TokenInfo {
    TokenInfo {
        id: model.id,
        name: model.name.clone(),
        created_at: model.created_at.clone(),
        expires_at: rfc3339(model.expires_at),
    }
}

/// Stores a token and returns its listing entry.
pub async fn create<C: ConnectionTrait>(conn: &C, token: NewToken<'_>) -> Result<TokenInfo> {
    let model = ActiveModel {
        user_id: Set(token.user_id.to_string()),
        name: Set(token.name.to_string()),
        email: Set(token.email.map(str::to_string)),
        display_name: Set(token.display_name.map(str::to_string)),
        token_hash: Set(token.token_hash.to_string()),
        created_at: Set(token.created_at.to_string()),
        expires_at: Set(token.expires_at),
        ..Default::default()
    }
    .insert(conn)
    .await?;
    Ok(info(&model))
}

/// Stores a token unless `user` already holds `MAX_TOKENS_PER_USER`. Expired tokens are removed first.
///
/// The removal, the count and the insert run in one transaction. On Postgres, an advisory lock on the
/// user makes concurrent logins for the same account wait for each other. SQLite allows one writer at
/// a time, and the removal is the first write, so the count sees the writes of the others.
/// Returns `None` when the account is full.
pub async fn create_within_limit(
    conn: &DatabaseConnection,
    token: NewToken<'_>,
    now: i64,
) -> Result<Option<TokenInfo>> {
    let txn = conn.begin().await?;
    if txn.get_database_backend() == DbBackend::Postgres {
        let lock = Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtext($1))",
            [token.user_id.into()],
        );
        txn.execute_raw(lock).await?;
    }
    delete_expired(&txn, token.user_id, now).await?;
    if count(&txn, token.user_id).await? >= MAX_TOKENS_PER_USER {
        txn.rollback().await?;
        return Ok(None);
    }
    let stored = create(&txn, token).await?;
    txn.commit().await?;
    Ok(Some(stored))
}

/// Whether `user` already has a token with this name.
pub async fn name_taken<C: ConnectionTrait>(conn: &C, user: &str, name: &str) -> Result<bool> {
    let select = Tokens::find()
        .filter(Column::UserId.eq(user))
        .filter(Column::Name.eq(name));
    Ok(PaginatorTrait::count(select, conn).await? > 0)
}

/// The tokens of `user`, newest first.
pub async fn list(
    conn: &DatabaseConnection,
    user: &str,
    window: Window,
) -> Result<Page<TokenInfo>> {
    let select = Tokens::find()
        .filter(Column::UserId.eq(user))
        .order_by_desc(Column::Id);
    let (models, total) = fetch_window(select, conn, window).await?;
    Ok(page(models.iter().map(info).collect(), total, window))
}

/// Deletes a token of `user`. Returns `false` if it does not exist for that user.
pub async fn delete(conn: &DatabaseConnection, user: &str, id: i64) -> Result<bool> {
    let result = Tokens::delete_many()
        .filter(Column::Id.eq(id))
        .filter(Column::UserId.eq(user))
        .exec(conn)
        .await?;
    Ok(result.rows_affected > 0)
}

/// The owner of the token with this hash, if there is one and it has not expired by `now` (Unix
/// seconds).
pub async fn owner_of(
    conn: &DatabaseConnection,
    token_hash: &str,
    now: i64,
) -> Result<Option<TokenOwner>> {
    let model = Tokens::find()
        .filter(Column::TokenHash.eq(token_hash))
        .filter(Column::ExpiresAt.gt(now))
        .one(conn)
        .await?;
    Ok(model.map(|model| TokenOwner {
        user_id: model.user_id,
        email: model.email,
        display_name: model.display_name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DatabaseConfig;
    use crate::db::{connect, connect_config, migrate};
    use crate::store::everything;
    use crate::store::test_db;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn token<'a>(user: &'a str, hash: &'a str) -> NewToken<'a> {
        NewToken {
            user_id: user,
            name: "CLI on laptop",
            email: Some("alex@example.com"),
            display_name: Some("Alex"),
            token_hash: hash,
            created_at: "2026-11-11T08:00:00+00:00",
            expires_at: 4_102_444_800,
        }
    }

    #[tokio::test]
    async fn a_token_is_found_by_its_hash_and_belongs_to_its_user() {
        let conn = test_db().await;
        create(&conn, token("alex", "hash-1")).await.unwrap();

        let owner = owner_of(&conn, "hash-1", 0)
            .await
            .unwrap()
            .expect("token exists");
        assert_eq!(owner.user_id, "alex");
        assert_eq!(owner.email.as_deref(), Some("alex@example.com"));
        assert!(owner_of(&conn, "other-hash", 0).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn a_token_is_refused_once_it_has_expired() {
        let conn = test_db().await;
        let mut expiring = token("alex", "hash-1");
        expiring.expires_at = 1_000;
        create(&conn, expiring).await.unwrap();

        assert!(owner_of(&conn, "hash-1", 999).await.unwrap().is_some());
        assert!(owner_of(&conn, "hash-1", 1_000).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn expired_tokens_are_removed_for_their_owner_only() {
        let conn = test_db().await;
        let mut old = token("alex", "old");
        old.expires_at = 1_000;
        create(&conn, old).await.unwrap();
        let mut other_old = token("bob", "bob-old");
        other_old.expires_at = 1_000;
        create(&conn, other_old).await.unwrap();
        create(&conn, token("alex", "fresh")).await.unwrap();

        assert_eq!(delete_expired(&conn, "alex", 2_000).await.unwrap(), 1);
        assert_eq!(count(&conn, "alex").await.unwrap(), 1);
        assert_eq!(count(&conn, "bob").await.unwrap(), 1);
    }

    /// Starts `attempts` logins for `user` at the same time and returns how many were stored.
    async fn race_for_tokens(
        conn: Arc<DatabaseConnection>,
        user: &'static str,
        attempts: usize,
    ) -> u64 {
        let mut tasks = Vec::new();
        for n in 0..attempts {
            let conn = conn.clone();
            tasks.push(tokio::spawn(async move {
                let hash = format!("race-{n}");
                let stored = create_within_limit(
                    &conn,
                    NewToken {
                        user_id: user,
                        name: "CLI",
                        email: None,
                        display_name: None,
                        token_hash: &hash,
                        created_at: "2026-11-13T00:00:00+00:00",
                        expires_at: 4_102_444_800,
                    },
                    0,
                )
                .await
                .unwrap();
                stored.is_some()
            }));
        }
        let mut stored = 0;
        for task in tasks {
            if task.await.unwrap() {
                stored += 1;
            }
        }
        stored
    }

    /// Simultaneous logins on a real SQLite file: the limit holds with several connections in use.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_logins_never_exceed_the_limit() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("t-rex-race-{}-{nanos}", std::process::id()));
        let path = dir.join("t-rex.db");
        let conn = connect_config(&DatabaseConfig::Sqlite(path)).await.unwrap();
        migrate(&conn).await.unwrap();
        let conn = Arc::new(conn);

        let stored = race_for_tokens(conn.clone(), "alex", 40).await;
        assert_eq!(stored, MAX_TOKENS_PER_USER);
        assert_eq!(count(&*conn, "alex").await.unwrap(), MAX_TOKENS_PER_USER);

        drop(conn);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The same race on Postgres, where the advisory lock does the serialising. Needs an empty database
    /// in `TEST_POSTGRES_URL`; run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn concurrent_logins_never_exceed_the_limit_on_postgres() {
        let url = std::env::var("TEST_POSTGRES_URL").expect("TEST_POSTGRES_URL is set");
        let conn = Arc::new(connect(&url).await.unwrap());
        migrate(&conn).await.unwrap();

        let stored = race_for_tokens(conn.clone(), "tester", 40).await;
        assert_eq!(stored, MAX_TOKENS_PER_USER);
        assert_eq!(count(&*conn, "tester").await.unwrap(), MAX_TOKENS_PER_USER);
    }

    #[tokio::test]
    async fn tokens_are_listed_per_user_and_can_be_revoked() {
        let conn = test_db().await;
        let mine = create(&conn, token("alex", "hash-1")).await.unwrap();
        create(&conn, token("bob", "hash-2")).await.unwrap();

        let listed = list(&conn, "alex", everything()).await.unwrap().items;
        assert_eq!(listed, vec![mine.clone()]);
        assert!(!delete(&conn, "bob", mine.id).await.unwrap());
        assert!(delete(&conn, "alex", mine.id).await.unwrap());
        assert!(owner_of(&conn, "hash-1", 0).await.unwrap().is_none());
    }
}
