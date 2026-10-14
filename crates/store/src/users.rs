use cairn_core::{User, UserId};
use jiff::{Span, Timestamp};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, sea_query::OnConflict};

use crate::{
  StoreError,
  entity::{login_state, user},
};

/// How long a login may take between the redirect to the provider and the callback.
const LOGIN_TTL_MINUTES: i64 = 10;

/// What a login in progress remembers for its callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginState {
  pub verifier: String,
  pub nonce: String,
  /// The editor address to return to after signing in.
  pub next: String,
}

fn into_user(row: user::Model) -> User {
  User {
    id: row.id.into(),
    email: row.email,
    name: row.name,
  }
}

/// The user for an identity provider's subject, created at the first login. Email and name follow the provider at every login.
pub async fn upsert_user(db: &DatabaseConnection, issuer: &str, subject: &str, email: &str, name: &str) -> Result<User, StoreError> {
  let now = Timestamp::now().to_string();
  let row = user::ActiveModel {
    id: Set(UserId::new().as_uuid()),
    issuer: Set(issuer.into()),
    subject: Set(subject.into()),
    // Lowercased copy, compared exactly: collation differs between the databases.
    email: Set(email.to_lowercase()),
    name: Set(name.into()),
    created_at: Set(now.clone()),
    last_login_at: Set(now),
  };
  user::Entity::insert(row)
    .on_conflict(
      OnConflict::columns([user::Column::Issuer, user::Column::Subject])
        .update_columns([user::Column::Email, user::Column::Name, user::Column::LastLoginAt])
        .to_owned(),
    )
    .exec(db)
    .await?;

  let row = user::Entity::find()
    .filter(user::Column::Issuer.eq(issuer))
    .filter(user::Column::Subject.eq(subject))
    .one(db)
    .await?
    .ok_or(StoreError::NotFound)?;

  Ok(into_user(row))
}

pub async fn get_user(db: &DatabaseConnection, id: UserId) -> Result<User, StoreError> {
  user::Entity::find_by_id(id.as_uuid()).one(db).await?.map(into_user).ok_or(StoreError::NotFound)
}

/// Remembers a login in progress under its `state` value, and forgets the ones nobody came back for.
pub async fn save_login_state(db: &DatabaseConnection, state: &str, login: &LoginState) -> Result<(), StoreError> {
  let now = Timestamp::now();
  let stale = now.checked_sub(Span::new().minutes(LOGIN_TTL_MINUTES)).map_err(|e| StoreError::Other(e.into()))?;

  login_state::Entity::delete_many().filter(login_state::Column::CreatedAt.lt(stale.to_string())).exec(db).await?;
  login_state::Entity::insert(login_state::ActiveModel {
    state: Set(state.into()),
    verifier: Set(login.verifier.clone()),
    nonce: Set(login.nonce.clone()),
    next: Set(login.next.clone()),
    created_at: Set(now.to_string()),
  })
  .exec(db)
  .await?;

  Ok(())
}

/// Takes the login a callback belongs to. A state works once: replaying a callback finds nothing.
pub async fn take_login_state(db: &DatabaseConnection, state: &str) -> Result<Option<LoginState>, StoreError> {
  let stale = Timestamp::now().checked_sub(Span::new().minutes(LOGIN_TTL_MINUTES)).map_err(|e| StoreError::Other(e.into()))?;
  let taken = login_state::Entity::delete_many().filter(login_state::Column::State.eq(state)).exec_with_returning(db).await?;

  Ok(taken.into_iter().find(|row| row.created_at.as_str() >= stale.to_string().as_str()).map(|row| LoginState {
    verifier: row.verifier,
    nonce: row.nonce,
    next: row.next,
  }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::connect::test_db;

  #[tokio::test]
  async fn a_user_is_found_again_by_issuer_and_subject() {
    let db = test_db().await;

    let first = upsert_user(&db, "https://idp.example", "abc", "Alice@Example.com", "Alice").await.unwrap();
    let again = upsert_user(&db, "https://idp.example", "abc", "alice@example.com", "Alice B.").await.unwrap();
    let other_provider = upsert_user(&db, "https://other.example", "abc", "alice@example.com", "Alice").await.unwrap();

    assert_eq!(first.id, again.id);
    assert_eq!((first.email.as_str(), again.name.as_str()), ("alice@example.com", "Alice B."));
    assert_ne!(first.id, other_provider.id);
  }

  #[tokio::test]
  async fn a_login_state_works_once() {
    let db = test_db().await;
    let login = LoginState {
      verifier: "v".into(),
      nonce: "n".into(),
      next: "/".into(),
    };

    save_login_state(&db, "s1", &login).await.unwrap();

    assert_eq!(take_login_state(&db, "s1").await.unwrap(), Some(login));
    assert_eq!(take_login_state(&db, "s1").await.unwrap(), None);
    assert_eq!(take_login_state(&db, "unknown").await.unwrap(), None);
  }
}
