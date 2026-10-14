SeaORM entities and migrations for SQLite and Postgres; the only crate with SQL.
Not obvious: IDs and timestamps are generated in Rust, never in SQL. `SqlContentStore` implements `ContentStore`: saves are conflict-checked against the draft's base revision.
Migrations are written once, in SQLite's spelling (`BLOB` ids, `INTEGER`, `STRICT`), and `migration::run` translates them for Postgres: keep to those spellings. Tests open databases with `connect_for_tests`, which uses Postgres when `TEST_DATABASE_URL` is set.
An API page's sources live in `api_configs` (JSON columns, read and written whole); a page that was never configured reads as an empty config. Users, sessions and API tokens store hashes of secrets, never the secrets.
Tree changes run in `write_tx` (SQLite `BEGIN IMMEDIATE`) with unique live slug indexes as the backstop; a publish reads one `staging()` snapshot. Page locks are leases (`locks.rs`, `expires_at` in milliseconds so SQL compares it).
