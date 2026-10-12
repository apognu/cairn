//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod assets;
mod connect;
mod content;
mod entity;
mod migration;
mod tree;

pub use assets::{NewAsset, asset_by_id, get_or_create_asset};
pub use connect::{StoreError, connect};
pub use content::SqlContentStore;
pub use migration::migrate;
pub use tree::{create_page, delete_page, list_pages, move_page, rename_page};
