# Migrations

This directory follows sqlx migration conventions.

- Add new migrations with `sqlx migrate add <name>` from `src-tauri/`.
- Keep migration files ordered and forward-only by default.
- Current baseline migration: `0001_create_anime_tables.sql`.
