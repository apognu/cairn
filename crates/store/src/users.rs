use cairn_core::{Role, User, UserId};
use jiff::{Span, Timestamp};
use sea_orm::{
  ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
  sea_query::{Expr, OnConflict, Query},
};

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

pub(crate) fn into_user(row: user::Model) -> Result<User, StoreError> {
  Ok(User {
    id: row.id.into(),
    email: row.email,
    name: row.name,
    role: parse_role(&row.role)?,
  })
}

pub(crate) fn parse_role(text: &str) -> Result<Role, StoreError> {
  Role::parse(text).ok_or_else(|| StoreError::Invalid(format!("unknown role {text}")))
}

/// The user for an identity provider's subject, created at the first login. Email and name follow the provider at every login; the role
/// is set once: admin for the very first user of the site, viewer for everyone after.
pub async fn upsert_user(db: &DatabaseConnection, issuer: &str, subject: &str, email: &str, name: &str) -> Result<User, StoreError> {
  let now = Timestamp::now().to_string();
  // Decided inside the insert, so two people signing in at once on a new site cannot both see it empty (on SQLite; Postgres could let both
  // through in the same instant, which leaves two admins and nothing worse).
  let first_is_admin = Expr::cust(r#"CASE WHEN EXISTS (SELECT 1 FROM "users") THEN 'viewer' ELSE 'admin' END"#);
  let insert = Query::insert()
    .into_table(user::Entity)
    .columns([
      user::Column::Id,
      user::Column::Issuer,
      user::Column::Subject,
      user::Column::Email,
      user::Column::Name,
      user::Column::Role,
      user::Column::CreatedAt,
      user::Column::LastLoginAt,
    ])
    .values([
      UserId::new().as_uuid().into(),
      issuer.into(),
      subject.into(),
      // Lowercased copy, compared exactly: collation differs between the databases.
      email.to_lowercase().into(),
      name.into(),
      first_is_admin,
      now.clone().into(),
      now.into(),
    ])
    .map_err(|e| StoreError::Other(e.into()))?
    .on_conflict(
      OnConflict::columns([user::Column::Issuer, user::Column::Subject])
        .update_columns([user::Column::Email, user::Column::Name, user::Column::LastLoginAt])
        .to_owned(),
    )
    .to_owned();
  db.execute(&insert).await?;

  let row = user::Entity::find()
    .filter(user::Column::Issuer.eq(issuer))
    .filter(user::Column::Subject.eq(subject))
    .one(db)
    .await?
    .ok_or(StoreError::NotFound)?;

  into_user(row)
}

pub async fn get_user(db: &DatabaseConnection, id: UserId) -> Result<User, StoreError> {
  user::Entity::find_by_id(id.as_uuid()).one(db).await?.map(into_user).ok_or(StoreError::NotFound)?
}

/// Changes a user's role. Refused with `Conflict` when it would leave the site without an admin: checked in the same statement, so two
/// admins demoting each other at once cannot both succeed on SQLite.
pub async fn set_user_role(db: &DatabaseConnection, id: UserId, role: Role) -> Result<User, StoreError> {
  let keeps_an_admin = match role {
    Role::Admin => Condition::all(),
    _ => Condition::any()
      .add(user::Column::Role.ne(Role::Admin.as_str()))
      .add(Expr::cust(r#"(SELECT COUNT(*) FROM "users" WHERE "role" = 'admin') > 1"#)),
  };
  let updated = user::Entity::update_many()
    .col_expr(user::Column::Role, Expr::value(role.as_str()))
    .filter(user::Column::Id.eq(id.as_uuid()))
    .filter(keeps_an_admin)
    .exec(db)
    .await?;

  let found = get_user(db, id).await?;
  if updated.rows_affected == 0 {
    return Err(StoreError::Conflict("the site needs at least one admin: make someone else admin first".into()));
  }

  Ok(found)
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
  async fn the_first_user_is_admin_and_the_next_ones_viewers() {
    let db = test_db().await;

    let first = upsert_user(&db, "https://idp.example", "a", "a@example.com", "A").await.unwrap();
    let second = upsert_user(&db, "https://idp.example", "b", "b@example.com", "B").await.unwrap();
    let first_again = upsert_user(&db, "https://idp.example", "a", "a@example.com", "A").await.unwrap();

    assert_eq!((first.role, second.role, first_again.role), (Role::Admin, Role::Viewer, Role::Admin));
  }

  #[tokio::test]
  async fn the_last_admin_cannot_be_demoted() {
    let db = test_db().await;
    let alice = upsert_user(&db, "https://idp.example", "a", "a@example.com", "A").await.unwrap().id;
    let bob = upsert_user(&db, "https://idp.example", "b", "b@example.com", "B").await.unwrap().id;

    assert!(matches!(set_user_role(&db, alice, Role::Editor).await, Err(StoreError::Conflict(_))));
    assert_eq!(set_user_role(&db, alice, Role::Admin).await.unwrap().role, Role::Admin);

    set_user_role(&db, bob, Role::Admin).await.unwrap();

    assert_eq!(set_user_role(&db, alice, Role::Viewer).await.unwrap().role, Role::Viewer);
    assert!(matches!(set_user_role(&db, bob, Role::Publisher).await, Err(StoreError::Conflict(_))));
    assert!(matches!(set_user_role(&db, UserId::new(), Role::Editor).await, Err(StoreError::NotFound)));
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
