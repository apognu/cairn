use jiff::{SignedDuration, Timestamp};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set};
use yourdocs_core::{ApiToken, UserId};

use crate::{
  StoreError,
  entity::{api_token, session},
};

/// How often a session's last-seen time is written: a request does not cost a write each.
const TOUCH_EVERY: SignedDuration = SignedDuration::from_secs(60);

/// Starts a session that ends at the latest `lifetime` from now. `token_hash` is the hash of the cookie's value.
pub async fn create_session(db: &DatabaseConnection, token_hash: &str, user: UserId, lifetime: SignedDuration) -> Result<(), StoreError> {
  let now = Timestamp::now();
  let expires = now.checked_add(lifetime).map_err(|e| StoreError::Other(e.into()))?;

  // Sessions nobody came back to would pile up otherwise.
  session::Entity::delete_many().filter(session::Column::ExpiresAt.lt(now.to_string())).exec(db).await?;
  session::ActiveModel {
    id_hash: Set(token_hash.into()),
    user_id: Set(user.as_uuid()),
    created_at: Set(now.to_string()),
    last_seen_at: Set(now.to_string()),
    expires_at: Set(expires.to_string()),
  }
  .insert(db)
  .await?;

  Ok(())
}

/// The user behind a session, if it is still valid: before its absolute end and not idle for longer than `idle`. A valid session counts
/// as seen now; an expired one is deleted.
pub async fn session_user(db: &DatabaseConnection, token_hash: &str, idle: SignedDuration) -> Result<Option<UserId>, StoreError> {
  let Some(row) = session::Entity::find_by_id(token_hash).one(db).await? else { return Ok(None) };

  let now = Timestamp::now();
  let parse = |text: &str| text.parse::<Timestamp>().map_err(|e| StoreError::Other(e.into()));
  let (expires, seen) = (parse(&row.expires_at)?, parse(&row.last_seen_at)?);

  if now >= expires || now.duration_since(seen) > idle {
    delete_session(db, token_hash).await?;
    return Ok(None);
  }

  if now.duration_since(seen) >= TOUCH_EVERY {
    let mut touched: session::ActiveModel = row.clone().into();
    touched.last_seen_at = Set(now.to_string());
    touched.update(db).await?;
  }

  Ok(Some(row.user_id.into()))
}

pub async fn delete_session(db: &DatabaseConnection, token_hash: &str) -> Result<(), StoreError> {
  session::Entity::delete_by_id(token_hash).exec(db).await?;

  Ok(())
}

fn into_token(row: api_token::Model) -> Result<ApiToken, StoreError> {
  let parse = |text: &str| text.parse::<Timestamp>().map_err(|e| StoreError::Other(e.into()));

  Ok(ApiToken {
    id: row.id,
    name: row.name,
    created_at: parse(&row.created_at)?,
    last_used_at: row.last_used_at.as_deref().map(parse).transpose()?,
  })
}

/// Stores a new token for a user. `token_hash` is the hash of the secret, which is not kept.
pub async fn create_api_token(db: &DatabaseConnection, user: UserId, name: &str, token_hash: &str) -> Result<ApiToken, StoreError> {
  let name = name.trim();
  if name.is_empty() || name.chars().count() > 80 {
    return Err(StoreError::Invalid("a token needs a name of up to 80 characters".into()));
  }

  let row = api_token::ActiveModel {
    id: Set(uuid::Uuid::now_v7()),
    user_id: Set(user.as_uuid()),
    name: Set(name.into()),
    token_hash: Set(token_hash.into()),
    created_at: Set(Timestamp::now().to_string()),
    last_used_at: Set(None),
  }
  .insert(db)
  .await?;

  into_token(row)
}

pub async fn list_api_tokens(db: &DatabaseConnection, user: UserId) -> Result<Vec<ApiToken>, StoreError> {
  let rows = api_token::Entity::find()
    .filter(api_token::Column::UserId.eq(user.as_uuid()))
    .order_by_asc(api_token::Column::CreatedAt)
    .all(db)
    .await?;

  rows.into_iter().map(into_token).collect()
}

/// Deletes one of a user's tokens; it stops working at once.
pub async fn delete_api_token(db: &DatabaseConnection, user: UserId, id: uuid::Uuid) -> Result<(), StoreError> {
  let deleted = api_token::Entity::delete_many()
    .filter(api_token::Column::Id.eq(id))
    .filter(api_token::Column::UserId.eq(user.as_uuid()))
    .exec(db)
    .await?;

  if deleted.rows_affected == 0 {
    return Err(StoreError::NotFound);
  }

  Ok(())
}

/// The user a token belongs to. Its last use is written at most once a minute.
pub async fn api_token_user(db: &DatabaseConnection, token_hash: &str) -> Result<Option<UserId>, StoreError> {
  let Some(row) = api_token::Entity::find().filter(api_token::Column::TokenHash.eq(token_hash)).one(db).await? else {
    return Ok(None);
  };

  let now = Timestamp::now();
  let stale = row
    .last_used_at
    .as_deref()
    .and_then(|t| t.parse::<Timestamp>().ok())
    .is_none_or(|used| now.duration_since(used) >= TOUCH_EVERY);
  if stale {
    let user = row.user_id;
    let mut touched: api_token::ActiveModel = row.into();
    touched.last_used_at = Set(Some(now.to_string()));
    touched.update(db).await?;

    return Ok(Some(user.into()));
  }

  Ok(Some(row.user_id.into()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{connect::test_db, upsert_user};

  /// Moves a session's clock back, as if it had been created and last used that long ago.
  async fn age(db: &DatabaseConnection, hash: &str, created: SignedDuration, seen: SignedDuration) {
    let mut row: session::ActiveModel = session::Entity::find_by_id(hash).one(db).await.unwrap().unwrap().into();
    let now = Timestamp::now();
    row.created_at = Set(now.checked_sub(created).unwrap().to_string());
    row.last_seen_at = Set(now.checked_sub(seen).unwrap().to_string());
    row.update(db).await.unwrap();
  }

  #[tokio::test]
  async fn a_session_ends_when_idle_or_past_its_lifetime_or_deleted() {
    let db = test_db().await;
    let user = upsert_user(&db, "https://idp.example", "a", "a@example.com", "A").await.unwrap().id;
    let hour = SignedDuration::from_hours(1);

    for hash in ["fresh", "idle", "old", "deleted"] {
      create_session(&db, hash, user, hour * 24).await.unwrap();
    }
    age(&db, "idle", hour * 3, hour * 2).await;
    delete_session(&db, "deleted").await.unwrap();
    let mut old: session::ActiveModel = session::Entity::find_by_id("old").one(&db).await.unwrap().unwrap().into();
    old.expires_at = Set((Timestamp::now() - hour).to_string());
    old.update(&db).await.unwrap();

    let idle = hour;
    assert_eq!(session_user(&db, "fresh", idle).await.unwrap(), Some(user));
    assert_eq!(session_user(&db, "idle", idle).await.unwrap(), None);
    assert_eq!(session_user(&db, "old", idle).await.unwrap(), None);
    assert_eq!(session_user(&db, "deleted", idle).await.unwrap(), None);
    assert_eq!(session_user(&db, "never-existed", idle).await.unwrap(), None);

    // The expired ones are gone, not just refused.
    assert_eq!(session::Entity::find().all(&db).await.unwrap().len(), 1);
  }

  #[tokio::test]
  async fn a_token_works_until_deleted_and_only_its_owner_can_delete_it() {
    let db = test_db().await;
    let alice = upsert_user(&db, "https://idp.example", "a", "a@example.com", "A").await.unwrap().id;
    let bob = upsert_user(&db, "https://idp.example", "b", "b@example.com", "B").await.unwrap().id;

    let token = create_api_token(&db, alice, "  CI  ", "hash-1").await.unwrap();

    assert_eq!(token.name, "CI");
    assert_eq!(api_token_user(&db, "hash-1").await.unwrap(), Some(alice));
    assert_eq!(api_token_user(&db, "other").await.unwrap(), None);
    assert!(list_api_tokens(&db, alice).await.unwrap()[0].last_used_at.is_some());
    assert!(list_api_tokens(&db, bob).await.unwrap().is_empty());
    assert!(matches!(delete_api_token(&db, bob, token.id).await, Err(StoreError::NotFound)));

    delete_api_token(&db, alice, token.id).await.unwrap();

    assert_eq!(api_token_user(&db, "hash-1").await.unwrap(), None);
    assert!(matches!(create_api_token(&db, alice, " ", "h").await, Err(StoreError::Invalid(_))));
  }
}
