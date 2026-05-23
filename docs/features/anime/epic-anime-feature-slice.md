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

## Phase Boundaries and Acceptance Criteria

### Phase 0 - Planning and Guardrails

Boundary:
- Architectural constraints, tracking, and naming are documented.

Acceptance criteria:
- Anime constraints captured as ADR.
- Epic/tracking document exists and references implementation docs.
- Milestone boundaries and phase acceptance criteria are defined.
- Naming conventions for commands, events, and model types are documented.

### Phase 1 - Feature Scaffolding

Boundary:
- Skeleton backend/frontend anime feature slice exists with command/event stubs.

Acceptance criteria:
- `src-tauri/src/services/anime/*` modules are created.
- Command stubs compile and are registered.
- Event names and payload placeholders are defined.
- `src/features/anime/*` folders exist with placeholder exports.

### Phase 2 - Canonical Models and Contracts

Boundary:
- Canonical anime models and typed contracts exist in Rust and TypeScript.

Acceptance criteria:
- All required models and error categories are implemented.
- Frontend mirror types match command/event payloads.
- Serialization/contract tests pass for Rust <-> TypeScript payloads.

### Phase 3 - Persistence and Cache Layer

Boundary:
- Anime data persistence and cache policy are in place.

Acceptance criteria:
- DB migrations for anime identity, metadata, mapping, progress, skip, settings are applied.
- Anime repositories are implemented and wired.
- TTL/invalidation policies are implemented and tested.

### Phase 4 - Classifier Service

Boundary:
- Anime classifier drives anime entry-path routing.

Acceptance criteria:
- Heuristic classifier returns `is_anime`, confidence, and reasons.
- Classifier integrated into detail/search entry flows.
- Unit tests cover positive, negative, and edge cases.

### Phase 5 - Metadata Enrichment Service

Boundary:
- AniList enrichment produces canonical anime details.

Acceptance criteria:
- TMDB metadata normalization and AniList reconciliation work.
- `anilist_id` and `mal_id` are persisted when available.
- Canonical titles/aliases are normalized.
- Tests pass for found, ambiguous, missing, and cached match scenarios.

### Phase 6 - Season and Episode Mapping

Boundary:
- Canonical season/episode graph is generated for movies and series.

Acceptance criteria:
- Mapping reconciles TMDB, AniList, and provider numbering.
- Unified `AnimeSeason` and `AnimeEpisode` outputs are used.
- Diagnostics exist for mapping decisions.
- Tests pass for linear, split/virtual, and fallback scenarios.

### Phase 7 - Provider Resolver

Boundary:
- Resolver pipeline returns canonical playback sources with typed failures.

Acceptance criteria:
- Provider interface, title candidates, matching, extraction, and normalization are implemented.
- Sub/dub fallback behavior and explicit translation errors are implemented.
- Compatibility overrides are isolated.
- Resolver test matrix passes.

### Phase 8 - Playback Session Orchestration

Boundary:
- Anime playback session lifecycle is orchestrated and persisted.

Acceptance criteria:
- Session state includes start/source/translation/progress/end.
- Typed lifecycle events are emitted.
- Shared playback subsystem modes are integrated.
- Progress persistence and watched-threshold behavior pass tests.

### Phase 9 - AniSkip Integration

Boundary:
- Skip timing resolution is integrated with cache and runtime modes.

Acceptance criteria:
- AniSkip fetch by `mal_id + episode_number` is implemented.
- Cache and canonical segment model are implemented.
- `off/auto/manual` modes function with proper gating.
- Skip events emit and boundary/no-data tests pass.

### Phase 10 - Subtitle and Download Preparation

Boundary:
- Anime subtitle normalization and download preparation are production usable.

Acceptance criteria:
- Subtitle candidates map to shared subtitle model.
- `anime_prepare_download` returns full canonical payload.
- Download queue integration validates with complete/failure tests.

### Phase 11 - Frontend Integration (Thin Client)

Boundary:
- Anime UI consumes typed backend contracts without provider coupling.

Acceptance criteria:
- Typed API wrappers, hooks, and components are implemented.
- Sub/dub toggle, episode selector, skip prompt, and error/fallback UI are functional.
- No provider-resolution logic exists in frontend components.
- Desktop/mobile responsive behavior is verified.

### Phase 12 - Settings, UX, and Diagnostics

Boundary:
- Anime runtime settings and diagnostics are configurable and observable.

Acceptance criteria:
- Settings for translation mode, skip mode, and subtitle language are persisted and loaded.
- Structured pipeline logging is implemented.
- Optional debug/verbose diagnostics mode is available.

### Phase 13 - Testing and Hardening

Boundary:
- Critical anime paths are validated and stable under load.

Acceptance criteria:
- Unit, integration, contract, frontend, and e2e smoke tests pass.
- Regression tests exist for compatibility overrides.
- Performance and soak checks meet baseline thresholds.

### Phase 14 - Rollout

Boundary:
- Anime feature can be safely enabled and monitored in production.

Acceptance criteria:
- Feature flag gate is functional.
- Internal QA across supported OS targets is complete.
- Development enablement and beta telemetry monitoring complete.
- General release occurs after stability threshold is met.

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
