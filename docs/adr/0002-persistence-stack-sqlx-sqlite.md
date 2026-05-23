# ADR 0002: Persistence Stack - sqlx + SQLite

## Status

Accepted

## Context

Ether requires a consistent persistence and migration workflow for backend-owned domains
including settings, library, progress, downloads, subtitles, and anime.

Current docs define SQLite usage, but do not pin the Rust data-access and migration stack.
Without a concrete choice, migration tooling and repository patterns can drift.

## Decision

Ether uses:

1. `SQLite` as the primary local structured database.
2. `sqlx` for Rust database access.
3. `sqlx` migrations under `src-tauri/migrations/`.

Migration conventions:

- Migration files live in `src-tauri/migrations/`.
- Use `sqlx migrate add <name>` for new migrations.
- Keep migrations forward-only by default.
- Existing baseline migration files remain valid when they follow sqlx ordering.

Runtime policy:

- App startup applies pending migrations before services initialize.
- Migration failures are surfaced as typed startup errors with actionable logs.

Repository policy:

- Repository modules own SQL queries and row-to-domain mapping.
- Service modules consume repositories and should not embed SQL directly.
- Domain models stay provider-agnostic and UI-agnostic.

Testing policy:

- Repository tests run against SQLite test databases.
- Migration tests validate schema bootstrapping from empty DB to latest.
- Contract tests continue to validate command/event serialization compatibility.

## Consequences

- Add `sqlx` and related runtime setup in `src-tauri`.
- Persistence work across features follows one migration mechanism.
- Future migration reviews can validate against this ADR.

## References

- `docs/ARCHITECUTRE.md`
- `docs/MIGRATION.md`
- `docs/features/anime/plan.md`
