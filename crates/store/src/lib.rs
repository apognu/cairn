//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod assets;
mod connect;
mod content;
mod entity;
mod migration;
mod redirects;
mod releases;
mod tree;
mod urls;

pub use assets::{NewAsset, asset_by_id, assets_by_ids, get_or_create_asset};
pub use connect::{StoreError, connect};
pub use content::SqlContentStore;
pub use migration::migrate;
pub use redirects::{draft_revisions, list_redirects};
pub use releases::{LiveRelease, add_release_pages, create_release, get_setting, latest_live_release, live_release, load_settings, save_settings, set_release_status, set_setting};
pub use sea_orm::DatabaseConnection;
pub use tree::{create_link, create_page, delete_page, edit_link, list_pages, move_page, rename_page};
pub use urls::{add_alias, list_aliases, normalize_path, remove_alias, set_custom_url};
