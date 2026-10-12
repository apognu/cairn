//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod connect;
mod content;
mod entity;
mod migration;
mod tree;

pub use connect::{StoreError, connect};
pub use content::SqlContentStore;
pub use migration::migrate;
pub use tree::{create_page, delete_page, list_pages, move_page, rename_page};
