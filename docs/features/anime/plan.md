# Anime Implementation Plan (Production Ready)

This plan is the execution checklist for building complete, production-ready anime support in Ether.

It follows the project architecture rules:

- Rust owns business logic.
- React renders state and invokes typed commands.
- Tauri commands/events are stable typed contracts.
- Provider-specific details do not leak into UI components.

This is not an MVP-only plan. The target is full anime capability parity-plus quality for real users.

## Production Readiness Goals

- [ ] Users can watch anime movies and episodes reliably.
- [ ] Users can switch sub/dub where available, with clear fallback behavior.
- [ ] Users can resume progress across sessions and devices (if sync exists later, keep model compatible).
- [ ] Users get intro/outro skip behavior with `off`, `auto`, and `manual` modes.
- [ ] Anime integrates with shared search, library, progress, subtitles, downloads, backups, and settings.
- [ ] Errors are typed, user-facing, and debuggable (no opaque resolver failures).
- [ ] Caching, persistence, and telemetry are in place for stable long-term operation.

## Phase 0 - Planning and Guardrails

- [x] Confirm architecture constraints in docs/ADR for anime.
- [x] Create epic/tracking issue: `Anime Feature Slice`.
- [x] Define milestone boundaries and acceptance criteria per phase.
- [x] Confirm naming conventions for commands, events, and model types.

## Phase 1 - Feature Scaffolding

- [x] Create Rust anime service modules:
  - [x] `src-tauri/src/services/anime/classifier`
  - [x] `src-tauri/src/services/anime/metadata`
  - [x] `src-tauri/src/services/anime/mapping`
  - [x] `src-tauri/src/services/anime/resolver`
  - [x] `src-tauri/src/services/anime/skip`
  - [x] `src-tauri/src/services/anime/playback`
- [x] Add command stubs:
  - [x] `anime_get_details`
  - [x] `anime_get_episode_list`
  - [x] `anime_resolve_playback`
  - [x] `anime_get_skip_timings`
  - [x] `anime_set_translation_mode`
  - [x] `anime_prepare_download`
- [x] Add event names and shared payload placeholders:
  - [x] `anime-playback-ready`
  - [x] `anime-playback-failed`
  - [x] `anime-progress-updated`
  - [x] `anime-skip-segment-active`
- [x] Create frontend feature folders:
  - [x] `src/features/anime/api`
  - [x] `src/features/anime/types`
  - [x] `src/features/anime/hooks`
  - [x] `src/features/anime/components`

## Phase 2 - Canonical Models and Contracts

- [x] Implement Rust models from docs:
  - [x] `AnimeIdentity`
  - [x] `AnimeDetails`
  - [x] `AnimeSeason`
  - [x] `AnimeEpisode`
  - [x] `AnimePlaybackRequest`
  - [x] `AnimePlaybackSource`
  - [x] `SkipSegment`
  - [x] `AnimeSkipTimings`
  - [x] `AnimeDownloadPayload`
  - [x] `AnimeSettings`
- [x] Implement structured error categories:
  - [x] `AnimeNotClassified`
  - [x] `AnilistMatchMissing`
  - [x] `SeasonMappingFailed`
  - [x] `ProviderSearchFailed`
  - [x] `ProviderEpisodeMissing`
  - [x] `PlayableSourceMissing`
  - [x] `TranslationUnavailable`
- [x] Mirror all command/event payloads in TypeScript types.
- [x] Add contract serialization tests for Rust <-> frontend payload compatibility.

## Phase 3 - Persistence and Cache Layer

Persistence stack note:

- SQLite + sqlx migrations/repositories (see `docs/adr/0002-persistence-stack-sqlx-sqlite.md`).

- [x] Add DB schema/migrations for anime domain:
  - [x] Anime identity cache
  - [x] AniList metadata cache
  - [x] Episode mapping cache
  - [x] Playback progress by canonical episode key
  - [x] Skip timings cache
  - [x] Translation preference settings
- [x] Create repositories:
  - [x] `AnimeRepository`
  - [x] `AnimeCacheRepository`
  - [x] `AnimeProgressRepository`
- [x] Define TTL and invalidation policy:
  - [x] AniList metadata TTL
  - [x] Provider lookup TTL
  - [x] Skip timings TTL

## Phase 4 - Classifier Service

- [x] Implement initial classifier compatibility heuristic:
  - [x] Animation genre signal
  - [x] Japanese language/origin signal
- [x] Return `is_anime`, confidence, and reasons.
- [x] Add unit tests for positive/negative and edge cases.
- [ ] Integrate classifier into detail/search entry flows.

## Phase 5 - Metadata Enrichment Service

- [x] Normalize TMDB input metadata.
- [x] Implement AniList lookup + identity reconciliation.
- [x] Persist `anilist_id` and `mal_id` when resolved.
- [x] Normalize titles (canonical/romaji/english/native/aliases).
- [x] Build and return canonical `AnimeDetails`.
- [x] Add tests for found/ambiguous/missing/cached match scenarios.

## Phase 6 - Season and Episode Mapping

- [x] Implement canonical mapping service for TMDB + AniList + provider numbering.
- [x] Return unified `AnimeSeason` and `AnimeEpisode` models.
- [x] Support movie and series mapping paths.
- [x] Add diagnostics for mapping decisions.
- [x] Add tests for:
  - [x] Linear seasons
  - [x] Split/virtual season behavior
  - [x] Missing episode metadata fallback

## Phase 7 - Provider Resolver

- [ ] Define provider resolver interface and pipeline steps.
- [ ] Implement backend title candidate generation from normalized aliases.
- [ ] Implement provider search and best-match selection.
- [ ] Implement source extraction/decoding and output normalization.
- [ ] Implement sub/dub fallback and explicit translation errors.
- [ ] Isolate hardcoded compatibility overrides in dedicated layer.
- [ ] Return canonical `AnimePlaybackSource`.
- [ ] Add tests for:
  - [ ] Sub successful resolution
  - [ ] Dub successful resolution
  - [ ] Dub fallback to sub
  - [ ] No source/error typing
  - [ ] Provider timeout/retry behavior
  - [ ] Title alias collision behavior

## Phase 8 - Playback Session Orchestration

- [ ] Implement anime playback session model:
  - [ ] Session start
  - [ ] Source selected
  - [ ] Translation mode
  - [ ] Progress updates
  - [ ] Session end
- [ ] Emit typed playback lifecycle events.
- [ ] Integrate with shared playback subsystem:
  - [ ] Remote webview-backed mode
  - [ ] Local proxy mode
  - [ ] Offline/local file mode
- [ ] Persist progress and apply watched-threshold rules.
- [ ] Add tests for resume behavior and mode switching.

## Phase 9 - AniSkip Integration

- [ ] Implement AniSkip fetch by `mal_id + episode_number`.
- [ ] Cache skip timings and expose canonical segments.
- [ ] Support modes:
  - [ ] `off`
  - [ ] `auto`
  - [ ] `manual`
- [ ] Gate skip activation by identity, seekability, and mode.
- [ ] Emit `anime-skip-segment-active` events for UI.
- [ ] Add tests for boundary timing and no-data behavior.

## Phase 10 - Subtitle and Download Preparation

- [ ] Normalize subtitle candidates to shared subtitle model.
- [ ] Implement `anime_prepare_download` backend path.
- [ ] Return full `AnimeDownloadPayload` with media URL, headers, identity, subtitle candidates, and filename metadata.
- [ ] Validate integration with shared download queue system.
- [ ] Add tests for payload completeness and failure paths.

## Phase 11 - Frontend Integration (Thin Client)

- [x] Implement typed command wrappers in `src/features/anime/api`.
- [x] Implement hooks:
  - [x] `useAnimeDetails`
  - [x] `useAnimeEpisodes`
  - [x] `useAnimePlaybackSession`
  - [x] `useAnimeSkipState`
- [x] Implement components:
  - [x] Anime labels/badges
  - [x] Episode selector
  - [x] Sub/dub toggle
  - [x] Skip prompt UI (manual mode)
  - [x] Playback error/fallback states
- [x] Ensure components contain no provider-resolution logic.
- [x] Verify responsive behavior on desktop and mobile layouts.

## Phase 12 - Settings, UX, and Diagnostics

- [ ] Add anime settings UI:
  - [ ] Default translation mode
  - [ ] Intro skip mode
  - [ ] Preferred subtitle language
- [ ] Persist and load settings through Rust settings service.
- [ ] Add structured logging across anime pipeline phases.
- [ ] Add optional debug/verbose flag for anime diagnostics.

## Phase 13 - Testing and Hardening

- [ ] Rust unit tests for each anime service.
- [ ] Rust integration tests for end-to-end resolve/playback flow.
- [ ] Contract tests for command/event payloads.
- [ ] Frontend tests for anime hooks and event handling.
- [ ] End-to-end smoke tests:
  - [ ] Anime movie playback
  - [ ] Anime series playback
  - [ ] Sub/dub switching
  - [ ] Skip modes (`off/auto/manual`)
  - [ ] Download preparation
- [ ] Regression tests for known compatibility overrides.
- [ ] Performance checks for resolver, mapping, and playback startup latency.
- [ ] Soak test for long-running playback sessions.

## Phase 14 - Rollout

- [ ] Gate feature behind `anime_feature_enabled` flag.
- [ ] Run internal QA checklist across supported OS targets.
- [ ] Enable for development builds.
- [ ] Run beta with telemetry and error monitoring.
- [ ] Release generally after stability threshold is met.

## Definition of Done (Anime Complete)

- [ ] Discovery: anime appears correctly in home/search/detail flows.
- [ ] Metadata: AniList enrichment and canonical anime identity are stable and cached.
- [ ] Episodes: canonical season/episode mapping is used end-to-end.
- [ ] Playback: anime movies and episodes play through supported provider flow.
- [ ] Translation: sub/dub switching works with explicit fallback/error states.
- [ ] Progress: resume and watched-state persistence works from canonical episode keys.
- [ ] AniSkip: `off/auto/manual` fully functional for eligible sources.
- [ ] Subtitles: subtitle candidate normalization and playback attachment work.
- [ ] Downloads: anime download payload preparation and queue integration work.
- [ ] Settings: anime settings persist and affect runtime behavior.
- [ ] Contracts: Rust commands/events and TS types remain in sync.
- [ ] Reliability: typed errors, logging, and diagnostics are implemented.
- [ ] Quality: unit/integration/e2e tests pass for anime critical paths.
- [ ] Rollout: feature flag, QA, telemetry, and release checklist completed.

## Milestone Order (Execution Sequence)

- [ ] Milestone 1: Scaffolding + contracts + persistence
- [ ] Milestone 2: Classifier + metadata enrichment
- [ ] Milestone 3: Mapping + resolver MVP
- [ ] Milestone 4: Playback orchestration + AniSkip
- [ ] Milestone 5: Subtitle/download integration + frontend
- [ ] Milestone 6: Hardening, QA, and rollout
