//! Uploads, image variants and asset metadata.

#![forbid(unsafe_code)]

mod sniff;
mod store;
mod variants;

pub use sniff::{ImageInfo, inspect, sha256_hex};
pub use store::{AssetBackend, GcsSettings, ObjectAssetStore, S3Settings};
pub use variants::{EncodedVariant, MAX_PIXELS, Processed, check_size, process};
