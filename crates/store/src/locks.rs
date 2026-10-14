//! Page locks: one editor per page, held by a lease the editor renews while its author is active.

use cairn_core::{LOCK_LEASE, LockToken, PageId, PageLock, UserId};
use jiff::Timestamp;
use sea_orm::{
  ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
  sea_query::{Expr, ExprTrait, OnConflict},
};

use crate::{StoreError, entity::page_lock};

/// Takes the page's lock for `user` with a new token: it succeeds only if the page is free, its lease ran out, or `user` holds it already
/// (another tab of theirs, which loses it). One atomic statement, so two editors arriving together cannot both win.
pub async fn acquire_lock(db: &DatabaseConnection, page: PageId, user: UserId, now: Timestamp) -> Result<(PageLock, LockToken), StoreError> {
  let token = LockToken::new();
  let expires = millis(now + LOCK_LEASE);
  let row = new_row(page, user, token, now);

  // The start time is kept when the same person comes back before the lease ran out (a reload), so "editing since" stays true.
  let acquired_at = format!(
    r#"CASE WHEN "page_locks"."user_id" = "excluded"."user_id" AND "page_locks"."expires_at" >= {now} THEN "page_locks"."acquired_at" ELSE "excluded"."acquired_at" END"#,
    now = millis(now),
  );
  let result = page_lock::Entity::insert(row)
    .on_conflict(
      OnConflict::column(page_lock::Column::PageId)
        .update_columns([page_lock::Column::UserId, page_lock::Column::Token, page_lock::Column::RenewedAt, page_lock::Column::ExpiresAt])
        .value(page_lock::Column::AcquiredAt, Expr::cust(acquired_at))
        .action_and_where(
          Expr::col((page_lock::Entity, page_lock::Column::ExpiresAt))
            .lt(millis(now))
            .or(Expr::col((page_lock::Entity, page_lock::Column::UserId)).eq(user.as_uuid())),
        )
        .to_owned(),
    )
    .exec_without_returning(db)
    .await?;

  let current = find(db, page).await?.ok_or(StoreError::NotFound)?;
  if result == 0 || current.token != token.as_uuid() {
    return Err(StoreError::LockHeld(into_lock(current)?));
  }
  debug_assert!(current.expires_at == expires);

  Ok((into_lock(current)?, token))
}

/// Extends the lease of the lock `token` names. Its lease may have run out, as long as nobody took the page since. Fails with the
/// current holder (`LockHeld`) or `NotFound` when the page has no lock any more.
pub async fn renew_lock(db: &impl ConnectionTrait, page: PageId, token: LockToken, now: Timestamp) -> Result<PageLock, StoreError> {
  let result = page_lock::Entity::update_many()
    .col_expr(page_lock::Column::RenewedAt, Expr::value(now.to_string()))
    .col_expr(page_lock::Column::ExpiresAt, Expr::value(millis(now + LOCK_LEASE)))
    .filter(page_lock::Column::PageId.eq(page.as_uuid()))
    .filter(page_lock::Column::Token.eq(token.as_uuid()))
    .exec(db)
    .await?;

  let current = find(db, page).await?;
  match current {
    Some(row) if result.rows_affected == 1 => into_lock(row),
    Some(row) if row.expires_at >= millis(now) => Err(StoreError::LockHeld(into_lock(row)?)),
    _ => Err(StoreError::NotFound),
  }
}

/// Gives the lock back; a token that no longer holds it changes nothing.
pub async fn release_lock(db: &DatabaseConnection, page: PageId, token: LockToken) -> Result<(), StoreError> {
  page_lock::Entity::delete_many()
    .filter(page_lock::Column::PageId.eq(page.as_uuid()))
    .filter(page_lock::Column::Token.eq(token.as_uuid()))
    .exec(db)
    .await?;

  Ok(())
}

/// The page's lock while its lease runs.
pub async fn page_lock(db: &impl ConnectionTrait, page: PageId, now: Timestamp) -> Result<Option<PageLock>, StoreError> {
  find(db, page).await?.filter(|row| row.expires_at >= millis(now)).map(into_lock).transpose()
}

/// Every lock whose lease runs.
pub async fn live_locks(db: &impl ConnectionTrait, now: Timestamp) -> Result<Vec<PageLock>, StoreError> {
  let rows = page_lock::Entity::find().filter(page_lock::Column::ExpiresAt.gte(millis(now))).all(db).await?;

  rows.into_iter().map(into_lock).collect()
}

/// Refuses a change to `page` while someone other than `user` holds its lock.
pub(crate) async fn check_unlocked(db: &impl ConnectionTrait, page: PageId, user: UserId, now: Timestamp) -> Result<(), StoreError> {
  match page_lock(db, page, now).await? {
    Some(lock) if lock.user_id != user => Err(StoreError::LockHeld(lock)),
    _ => Ok(()),
  }
}

/// Forgets the locks of pages that went to the trash.
pub(crate) async fn drop_locks(db: &impl ConnectionTrait, pages: &[PageId]) -> Result<(), StoreError> {
  page_lock::Entity::delete_many()
    .filter(page_lock::Column::PageId.is_in(pages.iter().map(|p| p.as_uuid())))
    .exec(db)
    .await?;

  Ok(())
}

/// Refuses a change to `page` while someone other than `user` holds its lock.
pub async fn ensure_unlocked(db: &DatabaseConnection, page: PageId, user: UserId) -> Result<(), StoreError> {
  check_unlocked(db, page, user, Timestamp::now()).await
}

async fn find(db: &impl ConnectionTrait, page: PageId) -> Result<Option<page_lock::Model>, StoreError> {
  Ok(page_lock::Entity::find_by_id(page.as_uuid()).one(db).await?)
}

fn new_row(page: PageId, user: UserId, token: LockToken, now: Timestamp) -> page_lock::ActiveModel {
  page_lock::ActiveModel {
    page_id: Set(page.as_uuid()),
    user_id: Set(user.as_uuid()),
    token: Set(token.as_uuid()),
    acquired_at: Set(now.to_string()),
    renewed_at: Set(now.to_string()),
    expires_at: Set(millis(now + LOCK_LEASE)),
  }
}

fn millis(at: Timestamp) -> i64 {
  at.as_millisecond()
}

fn into_lock(row: page_lock::Model) -> Result<PageLock, StoreError> {
  let parse = |s: &str| s.parse::<Timestamp>().map_err(|e| StoreError::Invalid(format!("bad timestamp: {e}")));

  Ok(PageLock {
    page_id: row.page_id.into(),
    user_id: row.user_id.into(),
    acquired_at: parse(&row.acquired_at)?,
    renewed_at: parse(&row.renewed_at)?,
    expires_at: Timestamp::from_millisecond(row.expires_at).map_err(|e| StoreError::Invalid(format!("bad lock expiry: {e}")))?,
  })
}

#[cfg(test)]
mod tests {
  use cairn_core::Placement;
  use jiff::SignedDuration;

  use super::*;
  use crate::{connect::test_db, create_page};

  /// Now, to the millisecond the lease is stored with.
  fn now() -> Timestamp {
    Timestamp::from_millisecond(Timestamp::now().as_millisecond()).unwrap()
  }

  #[tokio::test]
  async fn a_lock_is_held_until_its_lease_runs_out() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let page = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let start = now();
    let at = |secs: i64| start + SignedDuration::from_secs(secs);

    let (lock, token) = acquire_lock(&db, page, alice, at(0)).await.unwrap();
    assert_eq!(lock.expires_at, at(0) + LOCK_LEASE);

    // (who, when, wins): others wait for the lease; the holder may always come back.
    let cases = [(bob, 60, false), (alice, 90, true), (bob, 90 + 119, false), (bob, 90 + 121, true), (alice, 90 + 122, false)];
    for (who, secs, wins) in cases {
      let result = acquire_lock(&db, page, who, at(secs)).await;
      assert_eq!(result.is_ok(), wins, "{who} at {secs}s: {result:?}");
      if !wins {
        assert!(matches!(result, Err(StoreError::LockHeld(_))));
      }
    }

    // Alice's first token was replaced by her second acquire, and Bob holds the page now.
    assert!(matches!(renew_lock(&db, page, token, at(300)).await, Err(StoreError::LockHeld(l)) if l.user_id == bob));
    assert_eq!(page_lock(&db, page, at(300)).await.unwrap().unwrap().user_id, bob);
    assert!(page_lock(&db, page, at(90 + 121) + LOCK_LEASE + SignedDuration::from_secs(1)).await.unwrap().is_none());
  }

  #[tokio::test]
  async fn renewing_keeps_the_page_and_releasing_frees_it() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let page = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let now = now();

    let (first, token) = acquire_lock(&db, page, alice, now).await.unwrap();
    let later = now + SignedDuration::from_secs(100);
    let renewed = renew_lock(&db, page, token, later).await.unwrap();
    assert_eq!((renewed.acquired_at, renewed.expires_at), (first.acquired_at, later + LOCK_LEASE));
    assert!(
      acquire_lock(&db, page, bob, now + LOCK_LEASE + SignedDuration::from_secs(1)).await.is_err(),
      "the renewal moved the end"
    );

    release_lock(&db, page, LockToken::new()).await.unwrap();
    assert!(page_lock(&db, page, later).await.unwrap().is_some(), "a wrong token releases nothing");
    release_lock(&db, page, token).await.unwrap();
    assert!(matches!(renew_lock(&db, page, token, later).await, Err(StoreError::NotFound)));
    acquire_lock(&db, page, bob, later).await.unwrap();
  }

  #[tokio::test]
  async fn two_editors_arriving_together_get_one_lock() {
    let db = test_db().await;
    let page = create_page(&db, UserId::new(), None, "Guide", Placement::Last).await.unwrap().id;
    let now = Timestamp::now();

    let (a, b) = tokio::join!(acquire_lock(&db, page, UserId::new(), now), acquire_lock(&db, page, UserId::new(), now));

    assert_eq!(a.is_ok() as u8 + b.is_ok() as u8, 1, "{a:?} {b:?}");
  }

  #[tokio::test]
  async fn a_page_someone_else_edits_cannot_be_deleted() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let guide = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let child = create_page(&db, alice, Some(guide), "Install", Placement::Last).await.unwrap().id;
    acquire_lock(&db, child, alice, Timestamp::now()).await.unwrap();

    assert!(matches!(crate::delete_page(&db, guide, bob).await, Err(StoreError::LockHeld(_))));
    assert_eq!(crate::delete_page(&db, guide, alice).await.unwrap().len(), 2);
    assert!(live_locks(&db, Timestamp::now()).await.unwrap().is_empty());
  }
}
