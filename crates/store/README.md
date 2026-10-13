SeaORM entities and migrations for SQLite (Postgres later); the only crate with SQL.
Not obvious: IDs and timestamps are generated in Rust, never in SQL. `SqlContentStore` implements `ContentStore`: saves are conflict-checked against the draft's base revision.
An API page's sources live in `api_configs` (JSON columns, read and written whole); a page that was never configured reads as an empty config.
