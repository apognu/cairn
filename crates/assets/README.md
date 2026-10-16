# assets

Uploads and asset metadata: `ObjectAssetStore` (files by content hash in a folder, an S3 bucket or a GCS bucket, through `object_store`), `inspect` (type and size from the bytes) and `sha256_hex`.
Not obvious: the client's claimed type is never trusted; image variants and placeholders are not built yet (M2).
