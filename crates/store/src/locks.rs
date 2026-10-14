//! Page locks: one editor per page, held by a lease the editor renews while its author is active.

use cairn_core::{LOCK_IDLE, LOCK_LEASE, LockToken, PageId, PageLock, UserId};
use jiff::Timestamp;
use sea_orm::{
  ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
  sea_query::{Expr, ExprTrait, OnConflict},
};

use crate::{StoreError, connect::write_tx, entity::page_lock};

/// Takes the page's lock for `user` with a new token: it succeeds only if the page is free, its lease ran out, or `user` holds it in
/// another tab that has been idle for `LOCK_IDLE` (every editor tab opens a page on arrival; that must not stop the tab they are typing
/// in). One atomic statement, so two editors arriving together cannot both win. A new lock counts as idle until its holder does something.
pub async fn acquire_lock(db: &DatabaseConnection, page: PageId, user: UserId, now: Timestamp) -> Result<(PageLock, LockToken), StoreError> {
  let token = LockToken::new();
  let row = new_row(page, user, token, now);

  let result = page_lock::Entity::insert(row)
    .on_conflict(
      OnConflict::column(page_lock::Column::PageId)
        .update_columns([
          page_lock::Column::UserId,
          page_lock::Column::Token,
          page_lock::Column::AcquiredAt,
          page_lock::Column::RenewedAt,
          page_lock::Column::ExpiresAt,
          page_lock::Column::ActiveAt,
        ])
        .action_and_where(
          Expr::col((page_lock::Entity, page_lock::Column::ExpiresAt)).lt(millis(now)).or(
            Expr::col((page_lock::Entity, page_lock::Column::UserId))
              .eq(user.as_uuid())
              .and(Expr::col((page_lock::Entity, page_lock::Column::ActiveAt)).lt(millis(now - LOCK_IDLE))),
          ),
        )
        .to_owned(),
    )
    .exec_without_returning(db)
    .await?;

  let current = find(db, page).await?.ok_or(StoreError::NotFound)?;
  if result == 0 || current.token != token.as_uuid() {
    return Err(StoreError::LockHeld(into_lock(current)?));
  }

  Ok((into_lock(current)?, token))
}

/// Extends the lease of the lock `token` names, noting when its holder was last active. Its lease may have run out, as long as nobody
/// took the page since. Fails with the current holder (`LockHeld`) or `NotFound` when the page has no lock any more.
pub async fn renew_lock(db: &impl ConnectionTrait, page: PageId, token: LockToken, now: Timestamp, active_at: Timestamp) -> Result<PageLock, StoreError> {
  let result = page_lock::Entity::update_many()
    .col_expr(page_lock::Column::RenewedAt, Expr::value(now.to_string()))
    .col_expr(page_lock::Column::ExpiresAt, Expr::value(millis(now + LOCK_LEASE)))
    .col_expr(page_lock::Column::ActiveAt, Expr::value(millis(active_at.min(now))))
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

/// Takes the page from its holder with a new token, as long as they have been idle for `LOCK_IDLE` (or with `force`, at any time). Returns
/// the lock and whoever held it, so they can be told. An active holder without `force` is a `Conflict`.
pub async fn take_over_lock(db: &DatabaseConnection, page: PageId, user: UserId, now: Timestamp, force: bool) -> Result<(PageLock, LockToken, Option<UserId>), StoreError> {
  let tx = write_tx(db).await?;
  let previous = find(&tx, page).await?.filter(|row| row.expires_at >= millis(now) && row.user_id != user.as_uuid());

  if let Some(row) = &previous
    && !force
    && row.active_at > millis(now - LOCK_IDLE)
  {
    return Err(StoreError::Conflict("they are still working on the page".into()));
  }

  let token = LockToken::new();
  page_lock::Entity::insert(new_row(page, user, token, now))
    .on_conflict(
      OnConflict::column(page_lock::Column::PageId)
        .update_columns([
          page_lock::Column::UserId,
          page_lock::Column::Token,
          page_lock::Column::AcquiredAt,
          page_lock::Column::RenewedAt,
          page_lock::Column::ExpiresAt,
          page_lock::Column::ActiveAt,
        ])
        .to_owned(),
    )
    .exec_without_returning(&tx)
    .await?;
  let lock = into_lock(find(&tx, page).await?.ok_or(StoreError::NotFound)?)?;
  tx.commit().await?;

  Ok((lock, token, previous.map(|row| row.user_id.into())))
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
    // Opening a page is not working on it: the holder is idle until their editor reports a keystroke or a save.
    active_at: Set(0),
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
    active_at: Timestamp::from_millisecond(row.active_at).map_err(|e| StoreError::Invalid(format!("bad lock activity: {e}")))?,
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
    renew_lock(&db, page, token, at(10), at(10)).await.unwrap();

    // (who, when, wins): others wait for the lease (from Alice's renewal at 10s); Alice's other tab waits until this one is idle.
    let cases = [(bob, 60, false), (alice, 90, false), (bob, 129, false), (bob, 131, true), (alice, 132, false)];
    for (who, secs, wins) in cases {
      let result = acquire_lock(&db, page, who, at(secs)).await;
      assert_eq!(result.is_ok(), wins, "{who} at {secs}s: {result:?}");
      if !wins {
        assert!(matches!(result, Err(StoreError::LockHeld(_))));
      }
    }

    // Bob holds the page now: Alice's token is dead.
    assert!(matches!(renew_lock(&db, page, token, at(200), at(200)).await, Err(StoreError::LockHeld(l)) if l.user_id == bob));
    assert_eq!(page_lock(&db, page, at(200)).await.unwrap().unwrap().user_id, bob);
    assert!(page_lock(&db, page, at(131) + LOCK_LEASE + SignedDuration::from_secs(1)).await.unwrap().is_none());
  }

  #[tokio::test]
  async fn a_tab_that_only_opened_the_page_gives_way_to_the_same_users_other_tab() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let page = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let start = now();
    let at = |secs: i64| start + SignedDuration::from_secs(secs);

    let (_, opened) = acquire_lock(&db, page, alice, at(0)).await.unwrap();
    assert!(acquire_lock(&db, page, bob, at(1)).await.is_err(), "someone else still waits for the lease");
    let (_, typing) = acquire_lock(&db, page, alice, at(2)).await.unwrap();
    renew_lock(&db, page, typing, at(3), at(3)).await.unwrap();

    assert!(matches!(renew_lock(&db, page, opened, at(4), at(4)).await, Err(StoreError::LockHeld(_))));
    assert!(acquire_lock(&db, page, alice, at(5)).await.is_err(), "the tab she types in keeps the page");
  }

  #[tokio::test]
  async fn renewing_keeps_the_page_and_releasing_frees_it() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let page = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let now = now();

    let (first, token) = acquire_lock(&db, page, alice, now).await.unwrap();
    let later = now + SignedDuration::from_secs(100);
    let renewed = renew_lock(&db, page, token, later, later).await.unwrap();
    assert_eq!((renewed.acquired_at, renewed.expires_at), (first.acquired_at, later + LOCK_LEASE));
    assert!(
      acquire_lock(&db, page, bob, now + LOCK_LEASE + SignedDuration::from_secs(1)).await.is_err(),
      "the renewal moved the end"
    );

    release_lock(&db, page, LockToken::new()).await.unwrap();
    assert!(page_lock(&db, page, later).await.unwrap().is_some(), "a wrong token releases nothing");
    release_lock(&db, page, token).await.unwrap();
    assert!(matches!(renew_lock(&db, page, token, later, later).await, Err(StoreError::NotFound)));
    acquire_lock(&db, page, bob, later).await.unwrap();
  }

  #[tokio::test]
  async fn only_an_idle_holder_can_be_taken_over_without_forcing() {
    let db = test_db().await;
    let (alice, bob) = (UserId::new(), UserId::new());
    let page = create_page(&db, alice, None, "Guide", Placement::Last).await.unwrap().id;
    let start = now();
    let at = |secs: i64| start + SignedDuration::from_secs(secs);
    let (_, token) = acquire_lock(&db, page, alice, at(0)).await.unwrap();

    // Alice's editor renews at 100s, reporting her last keystroke at 90s.
    renew_lock(&db, page, token, at(100), at(90)).await.unwrap();
    assert!(matches!(take_over_lock(&db, page, bob, at(150), false).await, Err(StoreError::Conflict(_))));

    let (lock, bobs, from) = take_over_lock(&db, page, bob, at(90 + 121), false).await.unwrap();
    assert_eq!((lock.user_id, from), (bob, Some(alice)));
    assert!(matches!(renew_lock(&db, page, token, at(220), at(220)).await, Err(StoreError::LockHeld(_))), "alice's token is dead");

    // Forcing works on an active holder, and taking a free page names nobody.
    renew_lock(&db, page, bobs, at(230), at(230)).await.unwrap();
    assert_eq!(take_over_lock(&db, page, alice, at(231), true).await.unwrap().2, Some(bob));
    release_lock(&db, page, take_over_lock(&db, page, alice, at(232), false).await.unwrap().1).await.unwrap();
    assert_eq!(take_over_lock(&db, page, bob, at(233), false).await.unwrap().2, None);
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
