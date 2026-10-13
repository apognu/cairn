use jiff::{SignedDuration, Timestamp};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use yourdocs_core::UserId;

use crate::{StoreError, entity::session};

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
}
