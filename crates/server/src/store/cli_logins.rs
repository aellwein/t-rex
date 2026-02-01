//! One-time login codes of the command line client. A code is used up by deleting its row, so when two
//! requests try the same code, only one of them can succeed.

use anyhow::Result;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::cli_login_code::{ActiveModel, Column, Entity as Codes};
use crate::store::api_tokens::TokenOwner;

/// Stores a code by its hash and removes codes that have expired by `now` (Unix seconds).
pub async fn create(
    conn: &DatabaseConnection,
    code_hash: &str,
    owner: &TokenOwner,
    expires_at: i64,
    now: i64,
) -> Result<()> {
    Codes::delete_many()
        .filter(Column::ExpiresAt.lte(now))
        .exec(conn)
        .await?;
    ActiveModel {
        code_hash: Set(code_hash.to_string()),
        user_id: Set(owner.user_id.clone()),
        email: Set(owner.email.clone()),
        display_name: Set(owner.display_name.clone()),
        expires_at: Set(expires_at),
    }
    .insert(conn)
    .await?;
    Ok(())
}

/// Uses up the code with this hash. Returns the account it was issued for, or `None` if the code is
/// unknown, already used, or expired by `now`. The code is gone afterwards in every case.
pub async fn take(
    conn: &DatabaseConnection,
    code_hash: &str,
    now: i64,
) -> Result<Option<TokenOwner>> {
    let Some(model) = Codes::find_by_id(code_hash.to_string()).one(conn).await? else {
        return Ok(None);
    };
    let deleted = Codes::delete_by_id(code_hash.to_string())
        .exec(conn)
        .await?;
    if deleted.rows_affected != 1 || model.expires_at <= now {
        return Ok(None);
    }
    Ok(Some(TokenOwner {
        user_id: model.user_id,
        email: model.email,
        display_name: model.display_name,
    }))
}
