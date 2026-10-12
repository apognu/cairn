//! SeaORM entities and migrations; the only crate with SQL.

#![forbid(unsafe_code)]

mod connect;
mod migration;

pub use connect::{StoreError, connect};
pub use migration::migrate;
