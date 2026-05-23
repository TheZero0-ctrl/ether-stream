# Epic: Anime Feature Slice

## Epic ID

`anime-feature-slice`

## Status

Planned

## Objective

Deliver production-ready anime support in Ether with backend-first Rust ownership,
typed command/event contracts, canonical data models, and shared-system integration.

## Scope

- Anime classification and entry-point routing
- AniList enrichment and canonical identity
- Season/episode mapping across TMDB, AniList, and provider numbering
- Provider playback resolution with sub/dub fallback
- Playback orchestration and progress persistence
- AniSkip intro/outro integration (`off`, `auto`, `manual`)
- Subtitle normalization and download payload preparation
- Frontend thin-client integration (hooks/components)
- Hardening, diagnostics, and rollout gating

## Out of Scope

- Separate anime app shell
- UI-level provider parsing/resolution logic
- Non-canonical feature flows that bypass shared systems

## Milestones

1. Scaffolding, contracts, and persistence
2. Classifier and metadata enrichment
3. Mapping and resolver MVP
4. Playback orchestration and AniSkip
5. Subtitle/download integration and frontend
6. Hardening, QA, and rollout

## Acceptance Criteria

- Anime movies and episodes resolve and play reliably.
- Sub/dub switching works with explicit fallback/error states.
- Progress persists by canonical episode key and resumes correctly.
- Skip modes (`off`, `auto`, `manual`) operate with typed events.
- Shared systems (search, library, downloads, subtitles, backups) remain canonical.
- Contracts are typed and validated between Rust and TypeScript.
- Logging and typed errors are sufficient for debugging production failures.
- Feature is rollout-gated behind `anime_feature_enabled`.

## Tracking Checklist

- [ ] Phase 0 complete
- [ ] Phase 1 complete
- [ ] Phase 2 complete
- [ ] Phase 3 complete
- [ ] Phase 4 complete
- [ ] Phase 5 complete
- [ ] Phase 6 complete
- [ ] Phase 7 complete
- [ ] Phase 8 complete
- [ ] Phase 9 complete
- [ ] Phase 10 complete
- [ ] Phase 11 complete
- [ ] Phase 12 complete
- [ ] Phase 13 complete
- [ ] Phase 14 complete

## References

- `docs/features/anime/plan.md`
- `docs/features/anime/architecture.md`
- `docs/features/anime/data-model.md`
- `docs/features/anime/provider-flow.md`
- `docs/features/anime/playback.md`
