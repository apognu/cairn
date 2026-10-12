# core

Domain types and traits; no I/O.
Documents are ProseMirror JSON (`Node` keeps absent fields absent so they round-trip unchanged); IDs are UUIDv7 newtypes.
Traits here: `ContentStore`, `AssetStore`. `Renderer`, `Backup`, `SearchIndexer`, `Broadcast` arrive with the tasks that need them.
