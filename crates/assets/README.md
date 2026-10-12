# assets

Uploads and asset metadata: `LocalAssetStore` (files by content hash on disk, through `object_store`), `inspect` (type and size from the bytes) and `sha256_hex`.
Not obvious: the client's claimed type is never trusted; image variants and placeholders are not built yet (M2).
