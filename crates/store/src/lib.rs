//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod api;
mod assets;
mod connect;
mod content;
mod entity;
mod history;
mod library;
mod links;
mod migration;
mod redirects;
mod releases;
mod sessions;
mod tree;
mod urls;
mod users;

pub use api::{ReleaseSpec, get_api_config, list_api_configs, record_release_specs, release_specs, save_api_config};
pub use assets::{NewAsset, asset_by_hash, asset_by_id, assets_by_ids, get_or_create_asset};
#[cfg(any(test, feature = "testing"))]
pub use connect::connect_for_tests;
pub use connect::{StoreError, connect, connect_url};
pub use content::SqlContentStore;
pub use history::{HistoryEntry, Trashed, list_trash, page_history, restore_page};
pub use library::{Entry, delete_entry, list_entries, load_library, save_snippet, save_term, save_variable};
pub use links::{StoredLink, all_links};
pub use migration::migrate;
pub use redirects::{draft_docs, draft_revisions, list_redirects};
pub use releases::{
  LiveRelease, add_release_pages, create_release, get_release, get_setting, latest_live_release, live_release, load_settings, make_release_live, save_settings, set_release_deployment,
  set_release_status, set_setting,
};
pub use sea_orm::DatabaseConnection;
pub use sessions::{api_token_user, create_api_token, create_session, delete_api_token, delete_session, list_api_tokens, session_user};
pub use tree::{create_api_page, create_link, create_page, delete_page, edit_link, list_pages, move_page, rename_page, set_description, set_in_header};
pub use urls::{RedirectTarget, add_alias, add_redirect, list_aliases, normalize_path, remove_alias, remove_redirect, set_custom_url};
pub use users::{LoginState, get_user, save_login_state, take_login_state, upsert_user};
