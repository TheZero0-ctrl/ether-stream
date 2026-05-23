# Anime Architecture

## Architectural Position

Anime should be implemented as a dedicated feature slice with backend-first ownership.

The current Streambert implementation spreads anime logic across:

- renderer helpers
- TV page state
- movie page state
- Electron main-process IPC
- provider-specific special cases

Ether should make this more explicit.

## Runtime Responsibilities

### Frontend

Frontend responsibilities should be limited to:

- rendering anime cards, detail screens, and episode lists
- invoking typed backend commands
- subscribing to progress, skip, and playback events
- presenting source selection and sub/dub UI
- showing errors and fallback states

Frontend should not be responsible for:

- anime detection heuristics
- AniList reconciliation logic
- provider title candidate generation
- episode-number mapping rules
- skip timing resolution

### Rust Backend

Rust should own:

- anime classification
- anime metadata enrichment
- canonical title and ID reconciliation
- season/episode modeling
- provider resolution
- playback orchestration
- subtitle attachment and search preparation
- download payload preparation
- AniSkip timing fetch and cache

## Recommended Service Breakdown

### `services/anime/classifier`

Responsibilities:

- determine whether a TMDB item should enter the anime pipeline
- return confidence and reason information if useful

Initial compatibility behavior can mirror Streambert's heuristic:

- genre contains animation
- language or country indicates Japanese origin

Later this should be replaceable with a stronger rule set.

### `services/anime/metadata`

Responsibilities:

- fetch base metadata from TMDB-backed media data already loaded elsewhere
- enrich anime entries from AniList
- normalize description, score, genres, and alternate titles
- expose canonical anime detail payloads

### `services/anime/mapping`

Responsibilities:

- produce a canonical season/episode graph for anime
- reconcile TMDB numbering, AniList season structure, and provider-facing numbering
- replace UI-only slicing logic from Streambert

This is one of the most important differences from the old app.

### `services/anime/resolver`

Responsibilities:

- resolve stream providers for anime movies and episodes
- manage provider-specific title candidates
- manage hardcoded overrides only as a fallback layer
- return a canonical playback source result

### `services/anime/skip`

Responsibilities:

- fetch and cache AniSkip data
- expose intro/outro windows using a canonical skip model
- avoid coupling skip behavior directly to one page component

### `services/anime/playback`

Responsibilities:

- coordinate selected source, selected translation mode, progress, subtitles, and skip state
- emit typed events to the frontend
- integrate with the shared playback layer for local/remote media

## Shared Feature Boundaries

Anime should integrate with shared systems rather than bypass them.

Shared systems include:

- search
- library
- progress tracking
- downloads
- subtitles
- backups

Anime-specific services should adapt into those shared systems through canonical models.

## Suggested Command Surface

Example Tauri commands for anime:

- `anime_get_details`
- `anime_get_seasons`
- `anime_get_episode_list`
- `anime_resolve_playback`
- `anime_get_skip_timings`
- `anime_set_translation_mode`
- `anime_prepare_download`

## Suggested Event Surface

Example events:

- `anime-playback-resolved`
- `anime-progress-updated`
- `anime-skip-window-changed`
- `anime-subtitle-discovered`
- `anime-download-prepared`

## Non-Goals

Anime should not require:

- a fully separate UI application shell
- provider-specific payload shapes leaking into React
- page-level duplication between anime movies and anime series when a shared model can exist
