---
trigger: glob
globs: "**/migrations/*.sql"
description: Protect frozen SQLite migrations 01..08, enforce sequential additive migrations (09+), and forbid compile-time sqlx::query! macros.
---
# SQLite Migrations & Query Rules
- NEVER edit or rename frozen migrations `01` through `08` (`07_add_scanner_v3_columns.sql`, `08_add_zero_io_and_curation_intelligence.sql`).
- Place all new schema changes in sequential additive migrations (`09_add_omkf_folder_crates.sql`).
- Use dynamic `sqlx::query`, `sqlx::query_as`, and `sqlx::QueryBuilder` exclusively; never use `sqlx::query!` macros.
