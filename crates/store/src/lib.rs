//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod api;
mod assets;
mod cleanup;
mod connect;
mod content;
mod entity;
mod history;
mod library;
mod links;
mod locks;
mod migration;
mod purge;
mod redirects;
mod release_log;
mod releases;
mod reviews;
mod sessions;
mod staging;
mod tree;
mod urls;
mod users;

pub use api::{ReleaseSpec, get_api_config, list_api_configs, record_release_specs, release_specs, save_api_config};
pub use assets::{NewAsset, asset_by_hash, asset_by_id, assets_by_ids, get_or_create_asset};
pub use cleanup::{asset_hashes, delete_assets, unused_assets};
#[cfg(any(test, feature = "testing"))]
pub use connect::connect_for_tests;
pub use connect::{StoreError, connect, connect_url, ping};
pub use content::SqlContentStore;
pub use history::{HistoryEntry, Trashed, list_trash, page_history, restore_page, restore_title};
pub use library::{Entry, TermInput, delete_entry, list_entries, load_library, save_snippet, save_term, save_variable};
pub use links::{StoredLink, all_links};
pub use locks::{acquire_lock, ensure_unlocked, live_locks, page_lock, release_lock, renew_lock, take_over_lock};
pub use migration::migrate;
pub use purge::purge_trash;
pub use redirects::{draft_docs, draft_revisions, list_redirects};
pub use release_log::{ReleaseEntry, release_log};
pub use releases::{
  LiveRelease, ReleasePage, add_release_pages, create_release, get_release, get_setting, kept_render_keys, latest_live_release, latest_preview_release, live_release, load_settings, make_release_live,
  release_state_of, save_settings, set_release_deployment, set_release_render_keys, set_release_stats, set_release_status, set_setting,
};
pub use reviews::{StaleRow, list_users, mark_reviewed, page_review, set_page_review, stale_pages};
pub use sea_orm::DatabaseConnection;
pub use sessions::{api_token_user, create_api_token, create_session, delete_api_token, delete_session, list_api_tokens, session_user};
pub use staging::{Staging, staging};
pub use tree::{create_api_page, create_link, create_page, delete_page, edit_link, list_pages, move_page, rename_page, set_description, set_hidden, set_in_header};
pub use urls::{RedirectTarget, add_alias, add_redirect, list_aliases, normalize_path, remove_alias, remove_redirect, set_custom_url};
pub use users::{LoginState, get_user, save_login_state, set_user_role, take_login_state, upsert_user};
