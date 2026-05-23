# ADR 0001: Anime Architecture Constraints

## Status

Accepted

## Context

Ether anime support needs a stable architecture contract before implementation starts.
Current docs already define backend-first ownership, typed contracts, and shared-system
integration, but those constraints should be explicitly captured as an ADR so future
implementation phases can validate against one source of truth.

## Decision

Anime feature work must follow these constraints:

1. Rust owns anime business logic (classification, AniList enrichment, mapping, resolver,
   skip timings, playback orchestration, and download preparation).
2. React remains a thin client responsible for rendering state, user interaction, and
   invoking typed commands.
3. Tauri commands and events are the contract boundary and must stay typed and mirrored
   between Rust and TypeScript payload models.
4. Provider-specific response shapes and resolver details must not leak into UI components.
5. Anime must integrate with shared systems (search, library, progress, subtitles,
   downloads, backups) through canonical models rather than bypassing those systems.

## Consequences

- Anime services are implemented under `src-tauri/src/services/anime/*`.
- Frontend anime code in `src/features/anime/*` only consumes canonical payloads.
- Contract and serialization tests are required to prevent Rust/TypeScript drift.
- Any provider override logic must stay isolated in resolver-compatible layers, never in UI.

## References

- `docs/ARCHITECUTRE.md`
- `docs/features/anime/architecture.md`
- `docs/features/anime/data-model.md`
- `docs/features/anime/plan.md`
