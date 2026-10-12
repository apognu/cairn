//! Uploads, image variants and asset metadata.

#![forbid(unsafe_code)]

mod local;
mod sniff;
mod variants;

pub use local::LocalAssetStore;
pub use sniff::{ImageInfo, inspect, sha256_hex};
pub use variants::{EncodedVariant, MAX_PIXELS, Processed, check_size, process};
