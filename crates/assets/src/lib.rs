//! Uploads, image variants and asset metadata.

#![forbid(unsafe_code)]

mod local;
mod sniff;

pub use local::LocalAssetStore;
pub use sniff::{ImageInfo, inspect, sha256_hex};
